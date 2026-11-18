//! The Belief Nexus's translator as the serving every translator shares
//! (`translator-serving`) runs it: the bodies it writes for its public,
//! meta and emission lanes, and the deployment it applies at start.
//!
//! Every setting, the translator's own among them, is kept by the Nexus under
//! one configuration revision; the translator follows each Configured answer
//! the Nexus gives, and takes the Nexus's snapshot of its settings when it
//! starts and whenever an answer skips a revision. At start it applies the
//! deployment's settings through the Nexus's meta socket, as conditional
//! Configures from revision 0, and stops at the first one the Nexus already
//! holds; until then the meta lane waits.
//!
//! Hand-written: the translator holds a lock and a condition, which Ethos
//! cannot name.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use belief_nexus_wire::{meta_call, public_call, BeliefNexus, ValidAdmission};
use emission_belief_nexus as emission;
use meta_signal_belief_nexus as meta;
use nexus_exchange::SocketPaths;
use nexus_querying::Patience;
use shared_translator::placement::Place;
use shared_translator::{Caller, PeerBinding, PeerId, PeerIdentity, ProcessId, UserId};
use signal::FrameCapacity;
use signal_belief_nexus as public;
use translator_serving::{
    EmissionAnswering, MetaTranslating, NexusCalling, NexusFailure, PublicTranslating, Settling,
    TextBudgeting, Translator, TranslatorBootstrap, TranslatorError, ValidTextBudget,
};

use crate::configuration::{keeper, TranslatorConfiguration};
use crate::emission::Attempting;

/// The peer name the deployment binds the paired Nexus under.
const PAIRED_NEXUS_PEER: &str = "belief-nexus";

/// How often the Nexus's meta socket is tried again while it is not yet
/// answering at start, within the admission deadline.
const RESTORE_RETRY: Duration = Duration::from_millis(10);

/// The translator of one place: its settings as the Nexus answers them, and
/// whether its deployment has been applied.
#[derive(Clone)]
pub struct BeliefTranslator {
    shared: Arc<Shared>,
}

struct Shared {
    place: Place,
    admission: ValidAdmission,
    bootstrap_budget: ValidTextBudget,
    settings: Mutex<Settings>,
    deployed: Condvar,
}

struct Settings {
    configuration: TranslatorConfiguration,
    deployed: bool,
}

/// The translator's settings as ordinary text is read under them: the text
/// budget the Nexus answered, or the bootstrap's until it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeliefReady {
    text_budget: ValidTextBudget,
}

impl TextBudgeting for BeliefReady {
    fn text_budget(&self) -> ValidTextBudget {
        self.text_budget
    }
}

/// The translator a bootstrap starts, its deployment not yet applied.
impl TryFrom<&TranslatorBootstrap> for BeliefTranslator {
    type Error = &'static str;

    fn try_from(bootstrap: &TranslatorBootstrap) -> Result<Self, Self::Error> {
        Ok(Self {
            shared: Arc::new(Shared {
                place: bootstrap.place.clone(),
                admission: ValidAdmission::try_from(&bootstrap.admission)?,
                bootstrap_budget: ValidTextBudget::try_from(&bootstrap.text_limits)?,
                settings: Mutex::new(Settings {
                    configuration: TranslatorConfiguration::default(),
                    deployed: false,
                }),
                deployed: Condvar::new(),
            }),
        })
    }
}

impl BeliefTranslator {
    fn settings(&self) -> MutexGuard<'_, Settings> {
        self.shared
            .settings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn paths(&self) -> SocketPaths<BeliefNexus> {
        SocketPaths::from(&self.shared.place)
    }

    fn capacity(&self) -> FrameCapacity {
        FrameCapacity::from(self.shared.admission.frame_capacity)
    }

    /// The translator itself as the caller of the calls it makes of its own.
    fn own_caller() -> Caller {
        Caller {
            user_id: UserId(i64::from(nix::unistd::getuid().as_raw())),
            process_id: ProcessId(i64::from(std::process::id())),
        }
    }

    /// One call of the Nexus's meta socket, its one answer within one
    /// admission deadline.
    fn meta_exchange(&self, input: meta_call::Input) -> Result<meta_call::Output, TranslatorError> {
        let deadline = Instant::now()
            .checked_add(self.shared.admission.deadline)
            .ok_or_else(|| TranslatorError::Signal("meta call deadline overflow".into()))?;
        let mut answer: Option<meta_call::Output> = None;
        NexusCalling::<meta_call::Output>::call(
            input,
            &self.paths().meta,
            Patience::Until(deadline),
            self.capacity(),
            |response| {
                answer = Some(response);
                Ok(())
            },
        )?;
        answer.ok_or_else(|| TranslatorError::Signal("the Nexus ended the meta call unanswered".into()))
    }

    /// Takes the Nexus's snapshot of the translator's settings.
    fn restore(&self) -> Result<meta::ConfigurationRevision, TranslatorError> {
        match self.meta_exchange(meta_call::Input::RestoreTranslatorConfiguration)? {
            meta_call::Output::Restored(snapshot) => {
                let mut settings = self.settings();
                settings
                    .configuration
                    .restore(&snapshot)
                    .map_err(|fault| TranslatorError::Signal(fault.0))?;
                Ok(snapshot.configuration_revision)
            }
            meta_call::Output::Failed(message) => Err(TranslatorError::Signal(message)),
            meta_call::Output::Answered(_) => Err(TranslatorError::Signal(
                "the Nexus answered a restore with a meta answer".into(),
            )),
        }
    }

    /// Follows a Configured answer: the settings take it, or, when it skips a
    /// revision, the Nexus's snapshot. The answer stands when the settings
    /// stand at its revision after; it is refused `Unavailable` when they
    /// cannot.
    fn followed(&self, configured: &meta::ConfiguredState) -> bool {
        if configured.keeper != keeper(&configured.set) {
            let _ = self.restore();
            return false;
        }
        if self.settings().configuration.apply(configured).is_ok() {
            return true;
        }
        self.restore()
            .is_ok_and(|revision| revision.0 >= configured.configuration_revision.0)
    }

    /// Applies the deployment: the settings the place starts with, then the
    /// paired Nexus's sockets and its peer, each a Configure conditional on
    /// the revision before it, from revision 0. The first the Nexus already
    /// holds a later revision for ends it: the Nexus was configured before.
    /// The translator's settings are first restored from the Nexus, tried
    /// again while its meta socket is not yet answering, within the
    /// admission deadline. The meta lane waits until this is done.
    pub fn deploy(&self, mut sets: Vec<meta::Set>, nexus: &PeerIdentity) -> Result<(), TranslatorError> {
        let deadline = Instant::now() + self.shared.admission.deadline;
        loop {
            match self.restore() {
                Ok(_) => break,
                Err(_) if Instant::now() < deadline => std::thread::sleep(RESTORE_RETRY),
                Err(error) => return Err(error),
            }
        }
        let paths = self.paths();
        sets.extend([
            meta::Set::NexusSocket(meta::SocketPath(paths.public.to_string_lossy().into_owned())),
            meta::Set::MetaNexusSocket(meta::SocketPath(paths.meta.to_string_lossy().into_owned())),
            meta::Set::AllowedNexusPeers(public::PeerList(vec![PeerBinding {
                peer_id: PeerId(PAIRED_NEXUS_PEER.into()),
                peer_identity: nexus.clone(),
            }])),
        ]);
        for (revision, set) in sets.into_iter().enumerate() {
            let revision = i64::try_from(revision)
                .map_err(|_| TranslatorError::Signal("deployment revision overflow".into()))?;
            let query = meta::Input::Configure(meta::ConfigureRequest {
                configuration_revision_option: Some(meta::ConfigurationRevision(revision)),
                set,
            });
            let response = self.meta_exchange(Self::meta_call(Self::own_caller(), query))?;
            match self.meta_answered(response).map_err(TranslatorError::from)? {
                meta::Output::Configured(_) => {}
                meta::Output::ConfigureRefused(meta::ConfigurationRefusal::StaleRevision(_))
                    if revision == 0 =>
                {
                    break
                }
                other => {
                    return Err(TranslatorError::Signal(format!(
                        "deployment configuration failed: {other:?}"
                    )))
                }
            }
        }
        self.settings().deployed = true;
        self.shared.deployed.notify_all();
        Ok(())
    }
}

impl Translator for BeliefTranslator {
    type Component = BeliefNexus;
    type Ready = BeliefReady;

    fn ready(&self) -> Option<BeliefReady> {
        Some(BeliefReady {
            text_budget: self
                .settings()
                .configuration
                .text_budget()
                .unwrap_or(self.shared.bootstrap_budget),
        })
    }
}

impl PublicTranslating for BeliefTranslator {
    type PublicInput = public::Input;
    type PublicOutput = public::Output;
    type PublicCall = public_call::Input;
    type PublicResponse = public_call::Output;

    fn public_call(caller: Caller, input: public::Input) -> public_call::Input {
        public_call::Input::Call(public_call::Call_Data { caller, query: input })
    }

    fn public_answered(response: public_call::Output) -> Result<public::Output, NexusFailure> {
        match response {
            public_call::Output::Answered(output) => Ok(output),
            public_call::Output::Failed(message) => Err(NexusFailure(message)),
        }
    }
}

impl MetaTranslating for BeliefTranslator {
    type MetaInput = meta::Input;
    type MetaOutput = meta::Output;
    type NexusMetaInput = meta::Input;
    type MetaCall = meta_call::Input;
    type MetaResponse = meta_call::Output;

    /// Every meta query is the Nexus's, which keeps every setting; it is
    /// forwarded once the deployment is applied.
    fn settle(&self, input: meta::Input) -> Settling<meta::Output, meta::Input> {
        let mut settings = self.settings();
        while !settings.deployed {
            settings = self
                .shared
                .deployed
                .wait(settings)
                .unwrap_or_else(PoisonError::into_inner);
        }
        Settling::Forwarded(input)
    }

    fn meta_call(caller: Caller, input: meta::Input) -> meta_call::Input {
        meta_call::Input::Call(meta_call::Call_Data { caller, query: input })
    }

    fn meta_answered(&self, response: meta_call::Output) -> Result<meta::Output, NexusFailure> {
        match response {
            meta_call::Output::Answered(meta::Output::Configured(state)) => {
                if self.followed(&state) {
                    Ok(meta::Output::Configured(state))
                } else {
                    Ok(meta::Output::ConfigureRefused(
                        meta::ConfigurationRefusal::Unavailable,
                    ))
                }
            }
            meta_call::Output::Answered(output) => Ok(output),
            meta_call::Output::Restored(_) => Err(NexusFailure(
                "the Nexus answered a meta call with a restore".into(),
            )),
            meta_call::Output::Failed(message) => Err(NexusFailure(message)),
        }
    }

    /// The meta contract has no subscription, so nothing is announced.
    fn announced(&self, _answered: &meta::Output) -> Option<meta::Output> {
        None
    }
}

impl EmissionAnswering for BeliefTranslator {
    type Emission = emission::EmissionAttempt;
    type Outcome = emission::EmissionAnswer;

    /// The attempt carries the translator settings it is done under.
    fn emitted(&self, _ready: &BeliefReady, attempt: emission::EmissionAttempt) -> emission::EmissionAnswer {
        attempt.attempted()
    }
}
