use crate::{Daemon, DaemonBootstrap, EmissionPort, FileMemory};
use emission_belief_nexus as emission;
use meta_signal_belief_nexus as meta;
use signal::{
    ByteViewable, FrameBody, FrameReading, FrameWriting, Restorable, Signal, Signalizable,
};
use signal_belief_nexus as wire;
use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub struct SocketEmission {
    socket: PathBuf,
    capacity: signal::FrameCapacity,
    timeout: Duration,
}

impl SocketEmission {
    pub fn new(bootstrap: &meta::NexusBootstrap) -> Self {
        Self {
            socket: PathBuf::from(&(bootstrap.emission_translator_socket.0).0),
            capacity: signal::FrameCapacity::from(bootstrap.admission.byte_limit.0 as usize),
            timeout: Duration::from_millis(bootstrap.admission.timeout.0 as u64),
        }
    }
}

impl EmissionPort for SocketEmission {
    fn emit(&mut self, attempt: emission::EmissionAttempt) -> emission::EmissionAttemptOutcome {
        let fallback = emission::EmissionAttemptOutcome {
            configuration_revision: attempt.configuration_revision,
            response: emission_failure(&attempt.query),
        };
        let Ok(signal) = attempt.signalize() else {
            return fallback;
        };
        let Ok(mut stream) = UnixStream::connect(&self.socket) else {
            return fallback;
        };
        let _ = stream.set_read_timeout(Some(self.timeout));
        let _ = stream.set_write_timeout(Some(self.timeout));
        if stream
            .write_frame(&FrameBody::from(signal.bytes().to_vec()), self.capacity)
            .is_err()
        {
            return fallback;
        }
        let Ok(body) = stream.read_frame(self.capacity) else {
            return fallback;
        };
        Signal::<emission::EmissionAttemptOutcome>::from(Vec::<u8>::from(body))
            .restore()
            .unwrap_or(fallback)
    }
}

fn emission_failure(query: &emission::Query) -> emission::Response {
    match query {
        emission::Query::Snapshot(repository) => emission::Response::EmissionRefused(
            emission::EmissionRefusal::Source(emission::SourceRefusal {
                repository: repository.clone(),
                source_operation: emission::SourceOperation::ReadingMain,
                source_evidence: emission::SourceEvidence::MainUnavailable,
            }),
        ),
        emission::Query::Publish(value) => emission::Response::EmissionRefused(
            emission::EmissionRefusal::PublicationFailed(emission::PublicationRefusal {
                workspace: value.workspace.clone(),
                publication_operation: emission::PublicationOperation::StagingGeneration,
                publication_evidence: emission::PublicationEvidence::PermissionDenied,
            }),
        ),
        emission::Query::InspectPublication(value) => emission::Response::EmissionRefused(
            emission::EmissionRefusal::PublicationFailed(emission::PublicationRefusal {
                workspace: value.workspace.clone(),
                publication_operation: emission::PublicationOperation::InspectingPointer,
                publication_evidence: emission::PublicationEvidence::PermissionDenied,
            }),
        ),
        emission::Query::Clean(value) => emission::Response::CleanupRefused(wire::FileFailure {
            workspace: value.workspace.clone(),
            file_operation: wire::FileOperation::Cleaning,
            file_evidence: wire::FileEvidence::PermissionDenied,
        }),
    }
}

pub fn run(bootstrap: meta::NexusBootstrap) -> io::Result<()> {
    run_until(bootstrap, Arc::new(AtomicBool::new(false)))
}

pub fn run_until(bootstrap: meta::NexusBootstrap, stop: Arc<AtomicBool>) -> io::Result<()> {
    let daemon = Daemon::start_file(
        DaemonBootstrap {
            nexus: bootstrap.clone(),
        },
        SocketEmission::new(&bootstrap),
    )
    .map_err(|error| io::Error::other(format!("bootstrap: {error:?}")))?;
    let ordinary = bind_new(Path::new(&(bootstrap.ordinary_socket.0).0))?;
    let privileged = bind_new(Path::new(&(bootstrap.meta_socket.0).0))?;
    ordinary.set_nonblocking(true)?;
    privileged.set_nonblocking(true)?;
    let daemon = Arc::new(Mutex::new(daemon));
    let meta_daemon = Arc::clone(&daemon);
    let meta_stop = Arc::clone(&stop);
    let paired = bootstrap.paired_translator.clone();
    let admission = bootstrap.admission.clone();
    let meta_thread =
        thread::spawn(move || accept_meta(privileged, meta_daemon, meta_stop, paired, admission));
    let poll_daemon = Arc::clone(&daemon);
    let poll_stop = Arc::clone(&stop);
    let poll_thread = thread::spawn(move || poll(poll_daemon, poll_stop));
    while !stop.load(Ordering::Acquire) {
        match ordinary.accept() {
            Ok((stream, _)) => {
                let daemon = Arc::clone(&daemon);
                let stop = Arc::clone(&stop);
                let paired = bootstrap.paired_translator.clone();
                let admission = bootstrap.admission.clone();
                thread::spawn(move || {
                    let _ = serve_ordinary(stream, daemon, stop, &paired, &admission);
                });
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10))
            }
            Err(error) => return Err(error),
        }
    }
    let _ = meta_thread.join();
    let _ = poll_thread.join();
    let _ = std::fs::remove_file(&(bootstrap.ordinary_socket.0).0);
    let _ = std::fs::remove_file(&(bootstrap.meta_socket.0).0);
    Ok(())
}

fn accept_meta(
    listener: UnixListener,
    daemon: Arc<Mutex<Daemon<SocketEmission, FileMemory>>>,
    stop: Arc<AtomicBool>,
    paired: meta::PairedTranslator,
    admission: meta::Admission,
) {
    while !stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => {
                let daemon = Arc::clone(&daemon);
                let paired = paired.clone();
                let admission = admission.clone();
                thread::spawn(move || {
                    let _ = serve_meta(stream, daemon, &paired, &admission);
                });
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10))
            }
            Err(_) => return,
        }
    }
}

fn serve_ordinary(
    mut stream: UnixStream,
    daemon: Arc<Mutex<Daemon<SocketEmission, FileMemory>>>,
    stop: Arc<AtomicBool>,
    paired: &meta::PairedTranslator,
    admission: &meta::Admission,
) -> io::Result<()> {
    configure_stream(&stream, admission)?;
    let capacity = capacity(admission);
    let body = stream.read_frame(capacity).map_err(frame_error)?;
    let call = Signal::<wire::Call>::from(Vec::<u8>::from(body))
        .restore()
        .map_err(restore_error)?;
    let paired_transport = admit_ordinary_call(&stream, paired, &call.caller)?;
    let owner = {
        let daemon = daemon.lock().map_err(lock_error)?;
        if paired_transport { daemon.login_owner(&call.caller) } else { daemon.direct_owner(&call.caller) }
    };
    let Some(owner) = owner else {
        let response = wire::Response::Refused(wire::Refusal::NotAuthorized(call.caller));
        return write_signal(&mut stream, &response, capacity);
    };
    if matches!(call.query, wire::Query::Subscribe) {
        let subscription = daemon.lock().map_err(lock_error)?.open_subscription_as(call, owner);
        match subscription {
            Ok(subscription) => {
                write_subscription(&mut stream, &subscription.initial.response, admission)?;
                let result = 'subscription: loop {
                    if stop.load(Ordering::Acquire) {
                        break Ok(());
                    }
                    let events = daemon
                        .lock()
                        .map_err(lock_error)?
                        .drain(subscription.id)
                        .unwrap_or_default();
                    for event in events {
                        if let Err(error) =
                            write_subscription(&mut stream, &event.response, admission)
                        {
                            break 'subscription Err(error);
                        }
                    }
                    thread::sleep(Duration::from_millis(10));
                };
                daemon
                    .lock()
                    .map_err(lock_error)?
                    .unsubscribe(subscription.id);
                return result;
            }
            Err(response) => return write_signal(&mut stream, &response, capacity),
        }
    }
    let response = daemon.lock().map_err(lock_error)?.handle_as(call, owner);
    write_signal(&mut stream, &response, capacity)
}

fn serve_meta(
    mut stream: UnixStream,
    daemon: Arc<Mutex<Daemon<SocketEmission, FileMemory>>>,
    paired: &meta::PairedTranslator,
    admission: &meta::Admission,
) -> io::Result<()> {
    admit(&stream, paired)?;
    configure_stream(&stream, admission)?;
    let capacity = capacity(admission);
    let body = stream.read_frame(capacity).map_err(frame_error)?;
    let input = Signal::<meta::MetaTransportInput>::from(Vec::<u8>::from(body))
        .restore()
        .map_err(restore_error)?;
    let output = daemon.lock().map_err(lock_error)?.meta_transport(input);
    write_signal(&mut stream, &output, capacity)
}

fn poll(daemon: Arc<Mutex<Daemon<SocketEmission, FileMemory>>>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Acquire) {
        let wait = daemon
            .lock()
            .ok()
            .and_then(|daemon| daemon.polling_timeout())
            .and_then(|value| u64::try_from(value.0).ok())
            .unwrap_or(100);
        thread::sleep(Duration::from_millis(wait.max(1)));
        if let Ok(mut daemon) = daemon.lock() {
            daemon.poll_once();
        } else {
            return;
        }
    }
}

fn admit(stream: &UnixStream, paired: &meta::PairedTranslator) -> io::Result<()> {
    let credential =
        nix::sys::socket::getsockopt(stream, nix::sys::socket::sockopt::PeerCredentials)
            .map_err(io::Error::other)?;
    if i64::from(credential.uid()) == paired.user_id.0
        && i64::from(credential.gid()) == paired.group_id.0
        && i64::from(credential.pid()) == paired.process_id.0
    {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unpaired translator",
        ))
    }
}

fn admit_ordinary_call(stream: &UnixStream, paired: &meta::PairedTranslator, caller: &translator::Caller) -> io::Result<bool> {
    let credentials = translator::unix::peer_credentials(stream).map_err(io::Error::other)?;
    let paired_transport = credentials.user_id == paired.user_id
        && credentials.group_id == paired.group_id
        && credentials.process_id == paired.process_id;
    if paired_transport || (credentials.user_id == caller.user_id && credentials.process_id == caller.process_id) {
        Ok(paired_transport)
    } else {
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "declared caller does not match live peer"))
    }
}

fn configure_stream(stream: &UnixStream, admission: &meta::Admission) -> io::Result<()> {
    let timeout = Duration::from_millis(admission.timeout.0 as u64);
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))
}
fn capacity(admission: &meta::Admission) -> signal::FrameCapacity {
    signal::FrameCapacity::from(admission.byte_limit.0 as usize)
}
fn write_signal<T: Signalizable>(
    stream: &mut UnixStream,
    value: &T,
    capacity: signal::FrameCapacity,
) -> io::Result<()> {
    let signal = value
        .signalize()
        .map_err(|error| io::Error::other(format!("signal: {error:?}")))?;
    stream
        .write_frame(&FrameBody::from(signal.bytes().to_vec()), capacity)
        .map_err(frame_error)
}
fn write_subscription(
    stream: &mut UnixStream,
    response: &wire::Response,
    admission: &meta::Admission,
) -> io::Result<()> {
    let archived = response
        .signalize()
        .map_err(|error| io::Error::other(format!("signal: {error:?}")))?;
    if archived.bytes().len() > admission.byte_limit.0 as usize {
        let terminal = wire::Response::SubscriptionClosed(wire::CloseReason::QueueOverflow);
        let _ = write_signal(stream, &terminal, capacity(admission));
        return Err(io::Error::other("subscription queue overflow"));
    }
    stream
        .write_frame(
            &FrameBody::from(archived.bytes().to_vec()),
            capacity(admission),
        )
        .map_err(frame_error)
}
fn bind_new(path: &Path) -> io::Result<UnixListener> {
    if path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            path.display().to_string(),
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    UnixListener::bind(path)
}
fn frame_error(error: signal::FrameError) -> io::Error {
    io::Error::other(error.to_string())
}
fn restore_error(error: impl std::fmt::Debug) -> io::Error {
    io::Error::other(format!("restore: {error:?}"))
}
fn lock_error<T>(_: std::sync::PoisonError<T>) -> io::Error {
    io::Error::other("daemon lock poisoned")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exchange<Q: Signalizable, R>(path: &Path, query: &Q) -> R
    where
        Signal<R>: Restorable<R>,
    {
        let mut stream = UnixStream::connect(path).unwrap();
        let signal = query.signalize().unwrap();
        stream
            .write_frame(
                &FrameBody::from(signal.bytes().to_vec()),
                signal::FrameCapacity::default(),
            )
            .unwrap();
        let body = stream.read_frame(signal::FrameCapacity::default()).unwrap();
        Signal::<R>::from(Vec::<u8>::from(body)).restore().unwrap()
    }

    #[test]
    fn authenticated_binary_meta_and_ordinary_sockets_restore_state() {
        let directory = tempfile::tempdir().unwrap();
        let caller = translator::Caller {
            user_id: translator::UserId(i64::from(nix::unistd::getuid().as_raw())),
            process_id: translator::ProcessId(i64::from(std::process::id())),
        };
        let identity = translator::PeerBinding {
            peer_id: translator::PeerId("test".into()),
            peer_identity: translator::PeerIdentity { user_id: caller.user_id, process_id: caller.process_id },
        };
        let ordinary = directory.path().join("ordinary.sock");
        let privileged = directory.path().join("meta.sock");
        let bootstrap = meta::NexusBootstrap {
            ordinary_socket: meta::OrdinarySocket(meta::SocketPath(
                ordinary.to_string_lossy().into_owned(),
            )),
            meta_socket: meta::MetaSocket(meta::SocketPath(
                privileged.to_string_lossy().into_owned(),
            )),
            emission_translator_socket: meta::EmissionTranslatorSocket(meta::SocketPath(
                directory
                    .path()
                    .join("translator.sock")
                    .to_string_lossy()
                    .into_owned(),
            )),
            place: translator::Place {
                place_name: translator::PlaceName("server".into()),
                runtime: translator::Runtime(directory.path().to_string_lossy().into_owned()),
                state: translator::State(directory.path().to_string_lossy().into_owned()),
            },
            memory_file_name: meta::MemoryFileName("memory".into()),
            admission: meta::Admission {
                byte_limit: meta::ByteLimit(1_000_000),
                timeout: meta::Timeout(1_000),
            },
            paired_translator: meta::PairedTranslator {
                user_id: caller.user_id,
                group_id: translator::GroupId(i64::from(nix::unistd::getgid().as_raw())),
                process_id: caller.process_id,
            },
            bootstrap_owner: meta::BootstrapOwner(wire::Owner::Login(caller.user_id)),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let running = Arc::clone(&stop);
        let restart_bootstrap = bootstrap.clone();
        let server = thread::spawn(move || run_until(bootstrap, running).unwrap());
        for _ in 0..100 {
            if privileged.exists() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let layout = meta::Layout {
            owned_root: meta::OwnedRoot(wire::RelativePath(".belief".into())),
            generations_directory: meta::GenerationsDirectory(wire::RelativePath(
                "generations".into(),
            )),
            current_link: meta::CurrentLink(wire::RelativePath("current".into())),
            settings_file: meta::SettingsFile(wire::RelativePath(".codex/config.toml".into())),
            instructions_setting: meta::InstructionsSetting("model_instructions_file".into()),
            instructions_file: meta::InstructionsFile(wire::RelativePath("instructions.md".into())),
            skills_directory: meta::SkillsDirectory(wire::RelativePath(".agents/skills".into())),
        };
        let sets = vec![
            meta::Set::RepositoryPolling(meta::Timeout(20)),
            meta::Set::SubscriberCapacity(2),
            meta::Set::OrdinaryPeers(wire::PeerList(vec![identity.clone()])),
            meta::Set::MetaPeers(wire::PeerList(vec![identity.clone()])),
            meta::Set::LayoutOfHarness(meta::HarnessLayout { harness: wire::Harness::Codex, layout }),
            meta::Set::GenerationRetention(meta::RetentionPolicy::KeepAll),
            meta::Set::NexusFrameCapacity(meta::ByteLimit(1_000_000)),
            meta::Set::NexusTransportTimeout(meta::Timeout(1_000)),
            meta::Set::NexusSocket(meta::SocketPath(ordinary.to_string_lossy().into_owned())),
            meta::Set::MetaNexusSocket(meta::SocketPath(privileged.to_string_lossy().into_owned())),
            meta::Set::CallTimeout(meta::Timeout(1_000)),
            meta::Set::OutputLimit(meta::ByteLimit(1_000_000)),
            meta::Set::TextBudget(meta::Budget { node_limit: meta::NodeLimit(10_000), depth_limit: meta::DepthLimit(100) }),
            meta::Set::TranslatorFrameCapacity(meta::ByteLimit(1_000_000)),
            meta::Set::TranslatorTransportTimeout(meta::Timeout(1_000)),
            meta::Set::AllowedNexusPeers(wire::PeerList(vec![identity.clone()])),
            meta::Set::GitExecutable(wire::ExecutablePath("git".into())),
        ];
        for (revision, set) in sets.into_iter().enumerate() {
            let response: meta::MetaTransportOutput = exchange(
                &privileged,
                &meta::MetaTransportInput::Call(meta::MetaCall {
                    caller: caller.clone(),
                    query: meta::Query::Configure(meta::ConfigureRequest {
                        configuration_revision_option: Some(meta::ConfigurationRevision(
                            revision as i64,
                        )),
                        set,
                    }),
                }),
            );
            assert!(matches!(response, meta::MetaTransportOutput::Answered(meta::Response::Configured(_))));
        }
        let response: wire::Response = exchange(
            &ordinary,
            &wire::Call {
                caller: caller.clone(),
                query: wire::Query::Resolve(Vec::new()),
            },
        );
        assert!(matches!(response, wire::Response::Resolved(_)));
        stop.store(true, Ordering::Release);
        server.join().unwrap();

        let stop = Arc::new(AtomicBool::new(false));
        let running = Arc::clone(&stop);
        let server = thread::spawn(move || run_until(restart_bootstrap, running).unwrap());
        for _ in 0..100 {
            if ordinary.exists() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let response: wire::Response = exchange(
            &ordinary,
            &wire::Call {
                caller,
                query: wire::Query::Resolve(Vec::new()),
            },
        );
        assert!(matches!(response, wire::Response::Resolved(_)));
        stop.store(true, Ordering::Release);
        server.join().unwrap();
    }
}
