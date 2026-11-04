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
    let executable = std::fs::canonicalize("/proc/self/exe").unwrap();
    let identity = wire::PeerIdentity {
        peer_name: wire::PeerName("process-test".into()),
        user_id: wire::UserId(i64::from(nix::unistd::getuid().as_raw())),
        group_id: wire::GroupId(i64::from(nix::unistd::getgid().as_raw())),
        executable_path: wire::ExecutablePath(executable.to_string_lossy().into_owned()),
        contract_digest: wire::ContractDigest("process-test".into()),
    };
    let caller = wire::Caller {
        user_id: identity.user_id.clone(),
        group_id: identity.group_id.clone(),
        process_id: wire::ProcessId(i64::from(std::process::id())),
        executable_path: identity.executable_path.clone(),
        contract_digest: identity.contract_digest.clone(),
    };
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
            group_id: caller.group_id.clone(),
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
    let response: wire::Response = exchange(
        &ordinary,
        &wire::Call {
            caller,
            query: wire::Query::Resolve(Vec::new()),
        },
    );
    assert!(matches!(
        response,
        wire::Response::Refused(wire::Refusal::NotAuthorized(_))
    ));
    child.kill().unwrap();
    child.wait().unwrap();
}
