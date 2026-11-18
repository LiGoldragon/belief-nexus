//! The Belief Nexus's sockets, served by the serving every Nexus shares
//! (`nexus-serving`).
//!
//! The meta socket answers the paired translator, one exchange at a time;
//! the public socket answers the paired translator and every direct peer,
//! each exchange attempted on the state the [`Belief`] actor owns and
//! answered once, or for a Subscribe with the stream of the owner's
//! furnishings. Each external effect goes to the translator on the emission
//! lane. Repository polling stays the Nexus's own: a thread that waits the
//! configured interval and asks the actor to poll once.

use crate::{
    Daemon, DaemonBootstrap, EmissionPort, Event, FileMemory, Frame, PublicCall, Subscription,
};
use belief_nexus_wire::transport::NexusBootstrap;
use belief_nexus_wire::{meta_call, public_call, BeliefNexus, ValidAdmission};
use emission_belief_nexus as emission;
use meta_signal_belief_nexus as meta;
use nexus_serving::{
    ConnectionServing, EmissionLane, Emitting, Listeners, Listening, MetaAnswering, PeerAdmitting,
    PublicServing, Reply, SubscriptionClosing, TransportBounded,
};
use signal_belief_nexus as wire;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

/// The Belief Nexus's four socket paths in its place.
pub type SocketPaths = nexus_serving::SocketPaths<BeliefNexus>;

/// One public connection, served with the Dispatcher's bodies.
pub type PublicConnection = nexus_serving::PublicConnection<Dispatcher>;

/// The emission lane to the paired translator. A failure to emit is answered
/// as the effect's own refusal, as the Nexus answers any effect it could not
/// see done.
pub struct SocketEmission {
    lane: EmissionLane,
    admission: ValidAdmission,
}

impl SocketEmission {
    pub fn new(paths: &SocketPaths, admission: ValidAdmission) -> Self {
        Self {
            lane: EmissionLane::from(paths.translator.clone()),
            admission,
        }
    }
}

impl EmissionPort for SocketEmission {
    fn emit(&mut self, attempt: emission::EmissionAttempt) -> emission::EmissionAttemptOutcome {
        let fallback = emission::EmissionAttemptOutcome {
            configuration_revision: attempt.configuration_revision,
            output: emission_failure(&attempt.input),
        };
        self.lane
            .emit::<_, emission::EmissionAttemptOutcome>(
                &attempt,
                self.admission.deadline,
                self.admission.frame_capacity,
            )
            .unwrap_or(fallback)
    }
}

fn emission_failure(input: &emission::Input) -> emission::Output {
    match input {
        emission::Input::Snapshot(repository) => emission::Output::EmissionRefused(
            emission::EmissionRefusal::Source(emission::SourceRefusal {
                repository: repository.clone(),
                source_operation: emission::SourceOperation::ReadingMain,
                source_evidence: emission::SourceEvidence::MainUnavailable,
            }),
        ),
        emission::Input::Publish(value) => emission::Output::EmissionRefused(
            emission::EmissionRefusal::PublicationFailed(emission::PublicationRefusal {
                workspace: value.workspace.clone(),
                publication_operation: emission::PublicationOperation::StagingGeneration,
                publication_evidence: emission::PublicationEvidence::PermissionDenied,
            }),
        ),
        emission::Input::InspectPublication(value) => emission::Output::EmissionRefused(
            emission::EmissionRefusal::PublicationFailed(emission::PublicationRefusal {
                workspace: value.workspace.clone(),
                publication_operation: emission::PublicationOperation::InspectingPointer,
                publication_evidence: emission::PublicationEvidence::PermissionDenied,
            }),
        ),
        emission::Input::Clean(value) => emission::Output::CleanupRefused(wire::FileFailure {
            workspace: value.workspace.clone(),
            file_operation: wire::FileOperation::Cleaning,
            file_evidence: wire::FileEvidence::PermissionDenied,
        }),
    }
}

/// Who a public connection was admitted as: the paired translator, which
/// carries a Caller of the Nexus's login, or a direct peer, whose declared
/// Caller must be the connection's own and whose owner is its configured
/// binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicAuthority {
    Translator,
    Direct,
}

/// Who an exchange is answered for: the authority its connection was
/// admitted under, and the Caller the call carries.
pub type PublicGrant = (PublicAuthority, translator::Caller);

/// A subscription opened, with the announcements of every event recorded
/// from then on, or the refusal it is answered with once.
type OpenedSubscription = Result<(Subscription, tokio::sync::watch::Receiver<i64>), wire::Output>;

/// The public socket admits every peer: whether a direct peer is a
/// configured one is decided for each exchange, and an unconfigured one is
/// answered `Refused.NotAuthorized`.
pub struct Admitting;

impl PeerAdmitting for Admitting {
    type Authority = PublicAuthority;

    fn translator() -> PublicAuthority {
        PublicAuthority::Translator
    }

    fn peer(&self, _caller: &translator::Caller) -> Option<PublicAuthority> {
        Some(PublicAuthority::Direct)
    }
}

/// The transport the public socket is served within: the bootstrap's
/// admission deadline and frame capacity, as every Belief socket is.
pub struct Transport {
    admission: ValidAdmission,
}

impl TransportBounded for Transport {
    fn transport_timeout(&self) -> Duration {
        self.admission.deadline
    }

    fn frame_capacity(&self) -> signal::FrameCapacity {
        signal::FrameCapacity::from(self.admission.frame_capacity)
    }
}

/// The Nexus's state, owned by one actor: every input is one message,
/// handled one at a time. An attempt that emits waits for the translator in
/// place, so its worker is handed back to the runtime meanwhile.
#[derive(kameo::Actor)]
pub struct Belief {
    daemon: Daemon<SocketEmission, FileMemory>,
}

impl Belief {
    fn owner(
        &self,
        authority: PublicAuthority,
        caller: &translator::Caller,
    ) -> Option<wire::Owner> {
        match authority {
            PublicAuthority::Translator => self.daemon.login_owner(caller),
            PublicAuthority::Direct => self.daemon.direct_owner(caller),
        }
    }
}

#[kameo::messages]
impl Belief {
    /// Answers one public input other than Subscribe for its owner.
    #[message]
    fn attempt(
        &mut self,
        authority: PublicAuthority,
        call: PublicCall,
    ) -> Result<wire::Output, Infallible> {
        let Some(owner) = self.owner(authority, &call.caller) else {
            return Ok(wire::Output::Refused(wire::Refusal::NotAuthorized(
                call.caller,
            )));
        };
        Ok(tokio::task::block_in_place(|| {
            self.daemon.handle_as(call, owner)
        }))
    }

    /// Opens a subscription for its owner, with the announcements of every
    /// event recorded from now on.
    #[message]
    fn open_subscription(
        &mut self,
        authority: PublicAuthority,
        call: PublicCall,
    ) -> Result<OpenedSubscription, Infallible> {
        let Some(owner) = self.owner(authority, &call.caller) else {
            return Ok(Err(wire::Output::Refused(wire::Refusal::NotAuthorized(
                call.caller,
            ))));
        };
        let announced = self.daemon.announcements();
        Ok(tokio::task::block_in_place(|| {
            self.daemon
                .open_subscription_as(call, owner)
                .map(|subscription| (subscription, announced))
        }))
    }

    /// The events recorded for a subscription since it last drained, or
    /// `None` once it is closed.
    #[message]
    fn drain_events(&mut self, id: i64) -> Result<Option<Vec<Event>>, Infallible> {
        Ok(self.daemon.drain(id))
    }

    /// A subscription's stream was dropped.
    #[message]
    fn release_subscription(&mut self, id: i64) {
        self.daemon.unsubscribe(id);
    }

    /// Answers one meta exchange.
    #[message]
    fn answer_meta(&mut self, input: meta_call::Input) -> Result<meta_call::Output, Infallible> {
        Ok(tokio::task::block_in_place(|| {
            self.daemon.meta_transport(input)
        }))
    }

    /// The configured repository polling interval.
    #[message]
    fn polling_interval(&mut self) -> Result<Option<meta::Timeout>, Infallible> {
        Ok(self.daemon.polling_timeout())
    }

    /// Polls every registered repository once.
    #[message]
    fn poll_repositories(&mut self) {
        tokio::task::block_in_place(|| {
            self.daemon.poll_once();
        });
    }
}

/// A subscription's place among the Nexus's subscribers, given back when its
/// stream is dropped, however the subscription ended.
struct SubscriptionLease {
    belief: kameo::actor::ActorRef<Belief>,
    id: i64,
}

impl Drop for SubscriptionLease {
    fn drop(&mut self) {
        let _ = self
            .belief
            .tell(ReleaseSubscription { id: self.id })
            .try_send();
    }
}

/// The stream of one subscription: its opening frame, then each event
/// recorded for it, drained when an event is announced.
fn subscription_stream(
    belief: kameo::actor::ActorRef<Belief>,
    subscription: Subscription,
    announced: tokio::sync::watch::Receiver<i64>,
) -> signal::Stream<wire::Workspaces, wire::Furnishing> {
    use futures_util::StreamExt;
    let lease = SubscriptionLease {
        belief,
        id: subscription.id,
    };
    let opened = futures_util::stream::once(std::future::ready(subscription.initial.frame));
    let changes = futures_util::stream::unfold(
        (lease, announced, std::collections::VecDeque::<Frame>::new()),
        |(lease, mut announced, mut pending)| async move {
            loop {
                if let Some(frame) = pending.pop_front() {
                    return Some((frame, (lease, announced, pending)));
                }
                announced.changed().await.ok()?;
                let events = lease
                    .belief
                    .ask(DrainEvents { id: lease.id })
                    .await
                    .ok()??;
                pending.extend(events.into_iter().map(|event| event.frame));
            }
        },
    );
    let frames: std::pin::Pin<Box<dyn futures_util::Stream<Item = Frame> + Send>> =
        Box::pin(opened.chain(changes));
    signal::Stream::from(frames)
}

/// The public socket's bodies: every exchange is a Call carrying its Caller,
/// admitted under the connection's authority, and attempted on the state the
/// [`Belief`] actor owns.
#[derive(Clone)]
pub struct Dispatcher {
    belief: kameo::actor::ActorRef<Belief>,
    transport: Arc<Transport>,
}

impl Dispatcher {
    /// The actor ended, and no answer can be given: the Nexus ends, and its
    /// pair with it.
    fn ended(error: impl std::fmt::Display) -> ! {
        eprintln!("Belief Nexus actor ended: {error}");
        std::process::exit(1)
    }
}

impl PublicServing for Dispatcher {
    type Component = BeliefNexus;
    type Query = public_call::Input;
    type Response = public_call::Output;
    type Output = wire::Output;
    type State = wire::Workspaces;
    type Change = wire::Furnishing;
    type Configuration = Transport;
    type Authority = PublicAuthority;
    type Grant = PublicGrant;
    type Input = wire::Input;

    fn configuration(&self) -> Option<Arc<Transport>> {
        Some(Arc::clone(&self.transport))
    }

    /// The paired translator carries the Caller it observed; a direct peer's
    /// declared Caller must be the connection's own, or the connection ends
    /// unanswered.
    fn admitted(
        &self,
        authority: &PublicAuthority,
        caller: &translator::Caller,
        query: public_call::Input,
    ) -> Option<(PublicGrant, wire::Input)> {
        let public_call::Input::Call(call) = query;
        if *authority == PublicAuthority::Direct
            && (call.caller.user_id != caller.user_id
                || call.caller.process_id != caller.process_id)
        {
            eprintln!(
                "Belief Nexus public request failed: declared caller does not match live peer"
            );
            return None;
        }
        Some(((*authority, call.caller), call.query))
    }

    fn answer(
        &self,
        grant: PublicGrant,
        input: wire::Input,
        _configuration: Arc<Transport>,
    ) -> Reply<wire::Output, wire::Workspaces, wire::Furnishing> {
        let (authority, caller) = grant;
        let call = PublicCall {
            caller,
            query: input,
        };
        if matches!(call.query, wire::Input::Subscribe) {
            match self
                .belief
                .ask(OpenSubscription { authority, call })
                .blocking_send()
            {
                Ok(Ok((subscription, announced))) => Reply::Subscribed(subscription_stream(
                    self.belief.clone(),
                    subscription,
                    announced,
                )),
                Ok(Err(refusal)) => Reply::Once(refusal),
                Err(error) => Self::ended(error),
            }
        } else {
            match self.belief.ask(Attempt { authority, call }).blocking_send() {
                Ok(output) => Reply::Once(output),
                Err(error) => Self::ended(error),
            }
        }
    }

    fn carried(&self, _grant: &PublicGrant, _closing: &Arc<SubscriptionClosing>) -> bool {
        true
    }

    fn answered(output: wire::Output) -> public_call::Output {
        public_call::Output::Answered(output)
    }

    fn subscribed(frame: Frame) -> wire::Output {
        wire::Output::Subscribed(frame)
    }
}

/// The meta socket's body: each meta exchange answered by the actor.
pub struct MetaDispatcher {
    belief: kameo::actor::ActorRef<Belief>,
}

impl MetaAnswering for MetaDispatcher {
    type Query = meta_call::Input;
    type Response = meta_call::Output;

    fn answer(&self, input: meta_call::Input) -> meta_call::Output {
        match self.belief.ask(AnswerMeta { input }).blocking_send() {
            Ok(output) => output,
            Err(error) => meta_call::Output::Failed(error.to_string()),
        }
    }
}

/// Belief's own repository polling: wait the configured interval, then poll
/// every registered repository once.
fn poll(belief: kameo::actor::ActorRef<Belief>) {
    loop {
        let wait = belief
            .ask(PollingInterval)
            .blocking_send()
            .ok()
            .flatten()
            .and_then(|value| u64::try_from(value.0).ok())
            .unwrap_or(100);
        std::thread::sleep(Duration::from_millis(wait.max(1)));
        if belief.ask(PollRepositories).blocking_send().is_err() {
            return;
        }
    }
}

/// Starts the Nexus its bootstrap names and serves it until a listener fails
/// for good: the state is restored first, from the memory file in the
/// prepared place, then both sockets are bound; the meta socket and the
/// polling run on threads of their own, the public connections on the
/// runtime.
pub fn run(bootstrap: NexusBootstrap) -> Result<(), Box<dyn std::error::Error>> {
    let admission = ValidAdmission::try_from(&bootstrap.admission)
        .map_err(|message| format!("invalid bootstrap admission: {message}"))?;
    let paths = SocketPaths::from(&bootstrap.place);
    let daemon = Daemon::start_file(
        DaemonBootstrap {
            nexus: bootstrap.clone(),
        },
        SocketEmission::new(&paths, admission),
    )
    .map_err(|error| format!("bootstrap: {error:?}"))?;
    let listeners = Arc::new(Listeners::<BeliefNexus>::bind(
        &bootstrap.place,
        admission,
        bootstrap.peer_identity,
    )?);
    listeners.publish_public(listeners.stage_public()?);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let belief = {
        use kameo::actor::Spawn;
        let _entered = runtime.enter();
        Belief::spawn_with_mailbox(Belief { daemon }, kameo::mailbox::unbounded())
    };

    let meta = Arc::clone(&listeners);
    let meta_answering = MetaDispatcher {
        belief: belief.clone(),
    };
    std::thread::spawn(move || loop {
        if let Err(error) = meta.serve_next_meta(&meta_answering) {
            eprintln!("Belief Nexus meta listener failed permanently: {error}");
            std::process::exit(1);
        }
    });
    let polled = belief.clone();
    std::thread::spawn(move || poll(polled));

    let dispatcher = Dispatcher {
        belief,
        transport: Arc::new(Transport { admission }),
    };
    loop {
        let accepted = listeners
            .accept_public(&Admitting)
            .map_err(|error| format!("Belief Nexus public listener failed permanently: {error}"))?;
        runtime.spawn(PublicConnection::serve(
            accepted.stream,
            accepted.authority,
            dispatcher.clone(),
            Arc::clone(&dispatcher.transport),
        ));
    }
}
