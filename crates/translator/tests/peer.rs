use belief_nexus_translator::{admit_peer, decode_public, observe_peer, TextLimits};
use signal_belief_nexus as public;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::Command;

#[test]
fn child_peer_process() {
    let Ok(socket) = std::env::var("BELIEF_TEST_PEER_SOCKET") else { return };
    let _stream = UnixStream::connect(socket).expect("connect test peer");
    std::thread::sleep(std::time::Duration::from_millis(250));
}

#[test]
fn live_kernel_peer_identity_is_authority_and_digest_is_compatibility() {
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("peer.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_peer_process", "--nocapture"])
        .env("BELIEF_TEST_PEER_SOCKET", &socket)
        .spawn().unwrap();
    let (stream, _) = listener.accept().unwrap();
    let digest = public::ContractDigest("signal:compatible".into());
    let observed = observe_peer(&stream, digest.clone()).unwrap();
    assert_eq!(observed.process_id.0, i64::from(child.id()));
    let configured = shared_translator::PeerBinding { peer_id: shared_translator::PeerId("nexus".into()), peer_identity: shared_translator::PeerIdentity { user_id: observed.user_id.clone(), process_id: observed.process_id.clone() } };
    assert!(admit_peer(observed.clone(), &public::PeerList(vec![configured.clone()])).is_ok());
    let incompatible = shared_translator::PeerBinding { peer_id: configured.peer_id, peer_identity: shared_translator::PeerIdentity { user_id: observed.user_id.clone(), process_id: shared_translator::ProcessId(observed.process_id.0 + 1) } };
    assert!(admit_peer(observed, &public::PeerList(vec![incompatible])).is_err());
    child.wait().unwrap();
}

#[test]
fn caller_claim_is_not_a_public_text_query() {
    let claimed = public::Call {
        caller: shared_translator::Caller {
            user_id: shared_translator::UserId(0), process_id: shared_translator::ProcessId(1),
        },
        query: public::Query::Subscribe,
    };
    let text = shared_translator::encode(&claimed);
    assert!(decode_public(&text, TextLimits { nodes: 4096, depth: 64 }).is_err());
}
