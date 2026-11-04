use meta_signal_belief_nexus as meta;
use signal::{
    ByteViewable, FrameBody, FrameReading, FrameWriting, Restorable, Signal, Signalizable,
};
use signal_belief_nexus as wire;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
fn direct_peer_child() {
    let Some(socket) = std::env::var_os("BELIEF_DIRECT_PEER_SOCKET") else { return };
    let gate = std::path::PathBuf::from(std::env::var_os("BELIEF_DIRECT_PEER_GATE").unwrap());
    for _ in 0..500 {
        if gate.exists() { break; }
        thread::sleep(Duration::from_millis(2));
    }
    assert!(gate.exists(), "direct peer gate never opened");
    let caller = translator::Caller {
        user_id: translator::UserId(i64::from(nix::unistd::getuid().as_raw())),
        process_id: translator::ProcessId(i64::from(std::process::id())),
    };
    let response: wire::Response = exchange(
        Path::new(&socket),
        &wire::Call { caller, query: wire::Query::Resolve(Vec::new()) },
    );
    let authorized = std::env::var_os("BELIEF_DIRECT_PEER_AUTHORIZED").is_some();
    if authorized {
        assert!(!matches!(response, wire::Response::Refused(wire::Refusal::NotAuthorized(_))), "configured direct peer was not admitted: {response:?}");
    } else {
        assert!(matches!(response, wire::Response::Refused(wire::Refusal::NotAuthorized(_))), "unconfigured direct peer was admitted: {response:?}");
    }
}

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
fn nexus_process_serves_authenticated_meta_and_ordinary_signal() {
    let directory = tempfile::tempdir().unwrap();
    let caller = translator::Caller {
        user_id: translator::UserId(i64::from(nix::unistd::getuid().as_raw())),
        process_id: translator::ProcessId(i64::from(std::process::id())),
    };
    let identity = translator::PeerBinding { peer_id: translator::PeerId("process-test".into()), peer_identity: translator::PeerIdentity { user_id: caller.user_id.clone(), process_id: caller.process_id.clone() } };
    let ordinary = directory.path().join("ordinary.sock");
    let privileged = directory.path().join("meta.sock");
    let bootstrap = meta::NexusBootstrap {
        ordinary_socket: meta::OrdinarySocket(meta::SocketPath(
            ordinary.to_string_lossy().into_owned(),
        )),
        meta_socket: meta::MetaSocket(meta::SocketPath(privileged.to_string_lossy().into_owned())),
        emission_translator_socket: meta::EmissionTranslatorSocket(meta::SocketPath(
            directory
                .path()
                .join("translator.sock")
                .to_string_lossy()
                .into_owned(),
        )),
        memory_path: meta::MemoryPath(
            directory
                .path()
                .join("memory")
                .to_string_lossy()
                .into_owned(),
        ),
        admission: meta::Admission {
            byte_limit: meta::ByteLimit(1_000_000),
            timeout: meta::Timeout(1_000),
        },
        paired_translator: meta::PairedTranslator {
            user_id: caller.user_id.clone(),
            group_id: translator::GroupId(i64::from(nix::unistd::getgid().as_raw())),
            process_id: caller.process_id.clone(),
        },
        bootstrap_owner: meta::BootstrapOwner(identity.clone()),
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_belief-nexus-daemon"))
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let signal = bootstrap.signalize().unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_frame(
            &FrameBody::from(signal.bytes().to_vec()),
            signal::FrameCapacity::default(),
        )
        .unwrap();
    drop(child.stdin.take());
    for _ in 0..100 {
        if privileged.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let response: meta::MetaTransportOutput = exchange(
        &privileged,
        &meta::MetaTransportInput::Call(meta::MetaCall {
            caller: caller.clone(),
            query: meta::Query::Configure(meta::ConfigureRequest {
                configuration_revision_option: Some(meta::ConfigurationRevision(0)),
                set: meta::Set::RepositoryPolling(meta::Timeout(20)),
            }),
        }),
    );
    assert!(matches!(response, meta::MetaTransportOutput::Answered(meta::Response::Configured(_))));
    let current = std::env::current_exe().unwrap();
    let authorized_gate = directory.path().join("authorized.gate");
    let mut authorized = Command::new(&current)
        .args(["--exact", "direct_peer_child", "--nocapture"])
        .env("BELIEF_DIRECT_PEER_SOCKET", &ordinary)
        .env("BELIEF_DIRECT_PEER_GATE", &authorized_gate)
        .env("BELIEF_DIRECT_PEER_AUTHORIZED", "1")
        .spawn().unwrap();
    let binding = translator::PeerBinding {
        peer_id: translator::PeerId("flow-peer".into()),
        peer_identity: translator::PeerIdentity {
            user_id: translator::UserId(i64::from(nix::unistd::getuid().as_raw())),
            process_id: translator::ProcessId(i64::from(authorized.id())),
        },
    };
    let configured: meta::MetaTransportOutput = exchange(&privileged, &meta::MetaTransportInput::Call(meta::MetaCall {
        caller: caller.clone(),
        query: meta::Query::Configure(meta::ConfigureRequest {
            configuration_revision_option: Some(meta::ConfigurationRevision(1)),
            set: meta::Set::OrdinaryPeers(wire::PeerList(vec![binding])),
        }),
    }));
    assert!(matches!(configured, meta::MetaTransportOutput::Answered(meta::Response::Configured(_))));
    std::fs::write(&authorized_gate, b"go").unwrap();
    assert!(authorized.wait().unwrap().success(), "configured direct peer process failed");

    let refused_gate = directory.path().join("refused.gate");
    let mut refused = Command::new(&current)
        .args(["--exact", "direct_peer_child", "--nocapture"])
        .env("BELIEF_DIRECT_PEER_SOCKET", &ordinary)
        .env("BELIEF_DIRECT_PEER_GATE", &refused_gate)
        .spawn().unwrap();
    std::fs::write(&refused_gate, b"go").unwrap();
    assert!(refused.wait().unwrap().success(), "unconfigured direct peer process did not receive typed refusal");
    child.kill().unwrap();
    child.wait().unwrap();
}
