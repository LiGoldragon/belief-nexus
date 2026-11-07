//! Startup of the Nexus from a bootstrap: the envelope is admitted before the bootstrap is read,
//! the bootstrap is validated before any effect, and the memory file is selected at the prepared
//! Place under its exclusion before a socket is bound or the store is opened.

use belief_nexus_daemon::{
    effective_user, BootstrapError, Daemon, DaemonBootstrap, EmissionPort, FileMemory, MemoryImage,
    MemoryStore, PreparesMemory,
};
use emission_belief_nexus as emission;
use meta_signal_belief_nexus as meta;
use signal::{ByteViewable, FrameBody, FrameCapacity, FrameWriting, Signalizable};
use signal_belief_nexus as wire;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Tree {
    root: tempfile::TempDir,
}

impl Tree {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("temporary tree"),
        }
    }
    fn path(&self) -> &Path {
        self.root.path()
    }
    fn runtime(&self) -> PathBuf {
        self.path().join("run/nexus/belief")
    }
    fn state(&self) -> PathBuf {
        self.path().join("state/nexus/belief")
    }
    fn legacy(&self) -> PathBuf {
        self.path().join("state/metaflow/belief")
    }
}

fn user() -> translator::UserId {
    translator::UserId(effective_user())
}

fn place(runtime: &Path, state: &Path) -> translator::Place {
    translator::Place {
        place_name: translator::PlaceName("belief".into()),
        runtime: translator::Runtime(runtime.to_string_lossy().into_owned()),
        state: translator::State(state.to_string_lossy().into_owned()),
    }
}

fn bootstrap(place: translator::Place, file: &str) -> meta::NexusBootstrap {
    let runtime = PathBuf::from(&place.runtime.0);
    let socket = |name: &str| meta::SocketPath(runtime.join(name).to_string_lossy().into_owned());
    meta::NexusBootstrap {
        ordinary_socket: meta::OrdinarySocket(socket("ordinary.sock")),
        meta_socket: meta::MetaSocket(socket("meta.sock")),
        emission_translator_socket: meta::EmissionTranslatorSocket(socket("translator.sock")),
        place,
        memory_file_name: meta::MemoryFileName(file.into()),
        admission: meta::Admission {
            byte_limit: meta::ByteLimit(1_000_000),
            timeout: meta::Timeout(1_000),
        },
        paired_translator: meta::PairedTranslator {
            user_id: user(),
            group_id: translator::GroupId(0),
            process_id: translator::ProcessId(1),
        },
        bootstrap_owner: meta::BootstrapOwner(wire::Owner::Login(user())),
    }
}

fn standard(tree: &Tree, file: &str) -> meta::NexusBootstrap {
    bootstrap(place(&tree.runtime(), &tree.state()), file)
}

fn image_bytes(path: &Path) -> Vec<u8> {
    let image = MemoryImage {
        owner: wire::Owner::Login(user()),
        configuration: belief_nexus_daemon::Configuration {
            revision: meta::ConfigurationRevision(0),
            repository_polling: None,
            subscriber_capacity: None,
            ordinary_peers: None,
            meta_peers: None,
            layouts: Vec::new(),
            placements: Vec::new(),
            retention: meta::RetentionPolicy::KeepAll,
            nexus_frame_capacity: None,
            nexus_transport_timeout: None,
            translator_sets: Vec::new(),
        },
        repositories: Vec::new(),
        workspaces: Vec::new(),
        cleanup_maintenance: Vec::new(),
        next_event_sequence: 1,
    };
    FileMemory::new(path.to_path_buf())
        .store(&image)
        .expect("write a real store");
    std::fs::read(path).expect("read the store back")
}

struct NoEmission;

impl EmissionPort for NoEmission {
    fn emit(&mut self, _: emission::EmissionAttempt) -> emission::EmissionAttemptOutcome {
        unreachable!("startup emits nothing")
    }
}

fn is_root() -> bool {
    effective_user() == 0
}

#[test]
fn a_legacy_only_state_directory_is_moved_intact_and_the_real_store_is_loaded_not_replaced() {
    let tree = Tree::new();
    std::fs::create_dir_all(tree.legacy()).unwrap();
    let original = image_bytes(&tree.legacy().join("memory"));
    let value = standard(&tree, "memory");
    let path = value.prepared_memory(effective_user()).expect("prepared");
    assert_eq!(path, tree.state().join("memory"));
    assert!(!tree.legacy().exists(), "the legacy directory is gone");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "bytes preserved by the move"
    );
    let daemon = Daemon::start_file(DaemonBootstrap { nexus: value }, NoEmission)
        .expect("start over the moved store");
    drop(daemon);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "the store was loaded, not replaced by a fresh one"
    );
}

#[test]
fn a_sole_sema_file_is_renamed_to_the_current_name_and_a_repeat_changes_nothing() {
    let tree = Tree::new();
    std::fs::create_dir_all(tree.state()).unwrap();
    let original = image_bytes(&tree.state().join("memory.sema"));
    let value = standard(&tree, "memory");
    let path = value.prepared_memory(effective_user()).expect("selected");
    assert!(!tree.state().join("memory.sema").exists());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(
        value.prepared_memory(effective_user()).expect("repeat"),
        path
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn current_only_and_neither_select_the_current_path_without_creating_the_file() {
    let tree = Tree::new();
    let value = standard(&tree, "memory");
    let path = value.prepared_memory(effective_user()).expect("neither");
    assert!(tree.state().is_dir() && tree.runtime().is_dir());
    assert!(
        !path.exists(),
        "a fresh place makes no store before the daemon opens it"
    );
    let original = image_bytes(&path);
    assert_eq!(
        value
            .prepared_memory(effective_user())
            .expect("current only"),
        path
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn both_files_are_a_typed_conflict_and_neither_is_changed() {
    let tree = Tree::new();
    std::fs::create_dir_all(tree.state()).unwrap();
    std::fs::write(tree.state().join("memory"), b"current").unwrap();
    std::fs::write(tree.state().join("memory.sema"), b"legacy").unwrap();
    let error = standard(&tree, "memory")
        .prepared_memory(effective_user())
        .unwrap_err();
    assert!(matches!(error, BootstrapError::MemoryFile(_)), "{error:?}");
    assert_eq!(
        std::fs::read(tree.state().join("memory")).unwrap(),
        b"current"
    );
    assert_eq!(
        std::fs::read(tree.state().join("memory.sema")).unwrap(),
        b"legacy"
    );
}

#[test]
fn both_directories_are_a_typed_preparation_conflict_and_neither_is_changed() {
    let tree = Tree::new();
    std::fs::create_dir_all(tree.legacy()).unwrap();
    std::fs::create_dir_all(tree.state()).unwrap();
    std::fs::write(tree.legacy().join("memory"), b"legacy").unwrap();
    std::fs::write(tree.state().join("memory"), b"current").unwrap();
    let error = standard(&tree, "memory")
        .prepared_memory(effective_user())
        .unwrap_err();
    assert!(matches!(error, BootstrapError::Preparation(_)), "{error:?}");
    assert_eq!(
        std::fs::read(tree.legacy().join("memory")).unwrap(),
        b"legacy"
    );
    assert_eq!(
        std::fs::read(tree.state().join("memory")).unwrap(),
        b"current"
    );
}

#[test]
fn a_failed_selection_is_recovered_by_a_retry_with_the_bytes_intact() {
    if is_root() {
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let tree = Tree::new();
    std::fs::create_dir_all(tree.state()).unwrap();
    std::fs::write(tree.state().join("memory.sema"), b"legacy bytes").unwrap();
    std::fs::set_permissions(tree.state(), std::fs::Permissions::from_mode(0o555)).unwrap();
    let value = standard(&tree, "memory");
    let error = value.prepared_memory(effective_user());
    std::fs::set_permissions(tree.state(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        matches!(
            error,
            Err(BootstrapError::Preparation(_)) | Err(BootstrapError::MemoryFile(_))
        ),
        "{error:?}"
    );
    assert_eq!(
        std::fs::read(tree.state().join("memory.sema")).unwrap(),
        b"legacy bytes"
    );
    let path = value.prepared_memory(effective_user()).expect("retry");
    assert_eq!(std::fs::read(path).unwrap(), b"legacy bytes");
}

#[test]
fn concurrent_selections_of_one_standard_place_rename_once_and_none_fails() {
    let tree = Tree::new();
    std::fs::create_dir_all(tree.state()).unwrap();
    std::fs::write(tree.state().join("memory.sema"), b"legacy bytes").unwrap();
    let value = Arc::new(standard(&tree, "memory"));
    let succeeded = Arc::new(AtomicUsize::new(0));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let value = Arc::clone(&value);
            let succeeded = Arc::clone(&succeeded);
            std::thread::spawn(move || {
                value
                    .prepared_memory(effective_user())
                    .expect("no error, in particular no ENOENT");
                succeeded.fetch_add(1, Ordering::SeqCst);
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(succeeded.load(Ordering::SeqCst), 8);
    assert!(!tree.state().join("memory.sema").exists());
    assert_eq!(
        std::fs::read(tree.state().join("memory")).unwrap(),
        b"legacy bytes"
    );
}

#[test]
fn an_explicit_place_keeps_its_decoys_and_looks_for_no_legacy_directory() {
    let tree = Tree::new();
    let state = tree.path().join("custom-state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("decoy"), b"decoy").unwrap();
    std::fs::create_dir_all(tree.legacy()).unwrap();
    std::fs::write(tree.legacy().join("memory"), b"unrelated legacy").unwrap();
    let value = bootstrap(place(&tree.path().join("custom-run"), &state), "memory");
    let path = value.prepared_memory(effective_user()).expect("explicit");
    assert_eq!(path, state.join("memory"));
    assert_eq!(std::fs::read(state.join("decoy")).unwrap(), b"decoy");
    assert_eq!(
        std::fs::read(tree.legacy().join("memory")).unwrap(),
        b"unrelated legacy"
    );
}

#[test]
fn an_invalid_bootstrap_has_no_effect_through_the_library_either() {
    let tree = Tree::new();
    for file in ["", "..", "a/b", "memory.sema"] {
        let value = standard(&tree, file);
        assert!(
            matches!(
                value.prepared_memory(effective_user()),
                Err(BootstrapError::Refused(_))
            ),
            "{file:?}"
        );
        assert!(
            !tree.path().join("state").exists() && !tree.path().join("run").exists(),
            "{file:?}"
        );
        let started = Daemon::start_file(DaemonBootstrap { nexus: value }, NoEmission);
        assert!(started.is_err());
        assert!(
            !tree.path().join("state").exists(),
            "start_file shares the validator: {file:?}"
        );
    }
}

// The real daemon process.

fn spawn() -> Child {
    Command::new(env!("CARGO_BIN_EXE_belief-nexus-daemon"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn frame(child: &mut Child, bytes: Vec<u8>) {
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_frame(&FrameBody::from(bytes), FrameCapacity::default())
        .unwrap();
}

fn exits(child: &mut Child) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().unwrap() {
            return !status.success();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    false
}

fn nothing_made(tree: &Tree) -> bool {
    !tree.path().join("state").exists() && !tree.path().join("run").exists()
}

#[test]
fn a_bootstrap_without_the_right_envelope_is_refused_before_it_is_read() {
    let tree = Tree::new();
    let bytes = standard(&tree, "memory")
        .signalize()
        .unwrap()
        .bytes()
        .to_vec();
    let mut first = spawn();
    frame(&mut first, bytes.clone());
    frame(&mut first, bytes.clone());
    assert!(
        exits(&mut first),
        "the bootstrap frame sent as the envelope is refused"
    );
    let mut flipped = <meta::NexusBootstrap as meta::Enveloped>::envelope();
    flipped[0] ^= 1;
    let mut second = spawn();
    frame(&mut second, flipped);
    frame(&mut second, bytes);
    assert!(
        exits(&mut second),
        "a digest of another contract is refused"
    );
    let mut third = spawn();
    drop(third.stdin.take());
    assert!(exits(&mut third), "an empty pipe is refused");
    assert!(
        nothing_made(&tree),
        "no directory, socket or store was made"
    );
}

#[test]
fn an_invalid_direct_bootstrap_is_refused_with_no_directory_socket_or_store() {
    let tree = Tree::new();
    let envelope = <meta::NexusBootstrap as meta::Enveloped>::envelope();
    let mut outside = standard(&tree, "memory");
    outside.ordinary_socket = meta::OrdinarySocket(meta::SocketPath(
        tree.path()
            .join("elsewhere.sock")
            .to_string_lossy()
            .into_owned(),
    ));
    let mut other_owner = standard(&tree, "memory");
    other_owner.bootstrap_owner =
        meta::BootstrapOwner(wire::Owner::Login(translator::UserId(effective_user() + 1)));
    for value in [
        standard(&tree, ".."),
        standard(&tree, "memory.sema"),
        outside,
        other_owner,
    ] {
        let mut child = spawn();
        frame(&mut child, envelope.clone());
        frame(&mut child, value.signalize().unwrap().bytes().to_vec());
        assert!(exits(&mut child), "{value:?}");
    }
    assert!(nothing_made(&tree));
}

#[test]
fn a_valid_direct_bootstrap_does_prepare_and_bind() {
    let tree = Tree::new();
    let value = bootstrap(
        place(
            &tree.path().join("custom-run"),
            &tree.path().join("custom-state"),
        ),
        "memory",
    );
    let mut child = spawn();
    frame(
        &mut child,
        <meta::NexusBootstrap as meta::Enveloped>::envelope(),
    );
    frame(&mut child, value.signalize().unwrap().bytes().to_vec());
    let socket = tree.path().join("custom-run/ordinary.sock");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !socket.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let bound = socket.exists();
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        bound,
        "the positive control binds, so the no-effect checks above are not vacuous"
    );
    assert!(tree.path().join("custom-state/memory").exists());
}

/// The published 0.3.0 `NexusBootstrap`, field for field, from meta-signal-belief-nexus at
/// `9c90892` src/lib.rs. One representative retained old frame, sent where the envelope belongs.
mod previous {
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct SocketPath(pub String);
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct OrdinarySocket(pub SocketPath);
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct MetaSocket(pub SocketPath);
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct EmissionTranslatorSocket(pub SocketPath);
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct MemoryPath(pub String);
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct ByteLimit(pub i64);
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct Timeout(pub i64);
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct Admission {
        pub byte_limit: ByteLimit,
        pub timeout: Timeout,
    }
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct PairedTranslator {
        pub user_id: translator::UserId,
        pub group_id: translator::GroupId,
        pub process_id: translator::ProcessId,
    }
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct BootstrapOwner(pub signal_belief_nexus::Owner);
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct NexusBootstrap {
        pub ordinary_socket: OrdinarySocket,
        pub meta_socket: MetaSocket,
        pub emission_translator_socket: EmissionTranslatorSocket,
        pub memory_path: MemoryPath,
        pub admission: Admission,
        pub paired_translator: PairedTranslator,
        pub bootstrap_owner: BootstrapOwner,
    }
}

#[test]
fn a_retained_old_bootstrap_sent_as_the_first_frame_is_refused_before_any_effect() {
    let tree = Tree::new();
    let runtime = tree.runtime();
    let old = previous::NexusBootstrap {
        ordinary_socket: previous::OrdinarySocket(previous::SocketPath(
            runtime.join("ordinary.sock").to_string_lossy().into_owned(),
        )),
        meta_socket: previous::MetaSocket(previous::SocketPath(
            runtime.join("meta.sock").to_string_lossy().into_owned(),
        )),
        emission_translator_socket: previous::EmissionTranslatorSocket(previous::SocketPath(
            runtime
                .join("translator.sock")
                .to_string_lossy()
                .into_owned(),
        )),
        memory_path: previous::MemoryPath(
            tree.state().join("memory").to_string_lossy().into_owned(),
        ),
        admission: previous::Admission {
            byte_limit: previous::ByteLimit(1_000_000),
            timeout: previous::Timeout(1_000),
        },
        paired_translator: previous::PairedTranslator {
            user_id: user(),
            group_id: translator::GroupId(0),
            process_id: translator::ProcessId(1),
        },
        bootstrap_owner: previous::BootstrapOwner(wire::Owner::Login(user())),
    };
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&old)
        .unwrap()
        .to_vec();
    let mut child = spawn();
    frame(&mut child, bytes);
    assert!(exits(&mut child), "the old frame is not an envelope");
    assert!(
        nothing_made(&tree),
        "no directory, socket or store was made"
    );
}
