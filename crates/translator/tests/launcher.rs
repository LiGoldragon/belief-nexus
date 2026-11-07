//! The real `belief-pair` launcher: every input is validated before the place is prepared or a
//! child is spawned, the place is prepared before the children start, and when the Nexus child
//! fails the launcher fails and leaves no translator child running.

use belief_nexus_translator::{
    Deployment, MemoryFileName, NexusExecutable, SocketName, TranslatorExecutable,
};
use meta_signal_belief_nexus as meta;
use shared_translator::placement::{Place, PlaceName, Runtime, State};
use shared_translator::UserId;
use signal_belief_nexus as public;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Tree(tempfile::TempDir);

impl Tree {
    fn new() -> Self {
        Self(tempfile::tempdir().expect("temporary tree"))
    }
    fn path(&self) -> &Path {
        self.0.path()
    }
    fn marks(&self) -> PathBuf {
        let marks = self.path().join("marks");
        std::fs::create_dir_all(&marks).unwrap();
        marks
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
    fn script(&self, name: &str, body: &str) -> String {
        let path = self.path().join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path.to_string_lossy().into_owned()
    }
}

fn user() -> i64 {
    i64::from(nix::unistd::Uid::effective().as_raw())
}

fn place(tree: &Tree) -> Place {
    Place {
        place_name: PlaceName("belief".into()),
        runtime: Runtime(tree.runtime().to_string_lossy().into_owned()),
        state: State(tree.state().to_string_lossy().into_owned()),
    }
}

fn deployment(nexus: String, translator: String) -> Deployment {
    Deployment {
        nexus_executable: NexusExecutable(nexus),
        translator_executable: TranslatorExecutable(translator),
        ordinary_socket_name: SocketName("ordinary.sock".into()),
        meta_socket_name: SocketName("meta.sock".into()),
        ordinary_translator_socket_name: SocketName("translator.sock".into()),
        meta_translator_socket_name: SocketName("meta-translator.sock".into()),
        memory_file_name: MemoryFileName("memory".into()),
        admission: meta::Admission {
            byte_limit: meta::ByteLimit(1_000_000),
            timeout: meta::Timeout(1_000),
        },
        owner: public::Owner::Login(UserId(user())),
        set_vector: vec![],
    }
}

fn launch(place_text: &str, deployment_text: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_belief-pair"))
        .args([place_text, deployment_text])
        .output()
        .unwrap()
}

fn launch_values(place: &Place, deployment: &Deployment) -> Output {
    launch(
        &shared_translator::encode(place),
        &shared_translator::encode(deployment),
    )
}

/// A nexus child that records what it sees at its start, then ends as the Nexus does when its
/// memory file selection fails; and a translator child that stays.
fn children(tree: &Tree, nexus_ends: bool) -> (String, String) {
    let marks = tree.marks();
    let (state, runtime) = (tree.state(), tree.runtime());
    let nexus = tree.script(
        "nexus",
        &format!(
            "echo started > {m}/nexus-started\n[ -d {s} ] && echo yes > {m}/saw-state\n[ -d {r} ] && echo yes > {m}/saw-runtime\n[ -f {s}/memory ] && cp {s}/memory {m}/saw-memory\ncat > /dev/null\n{end}",
            m = marks.display(),
            s = state.display(),
            r = runtime.display(),
            end = if nexus_ends { "exit 3" } else { "exec sleep 60" }
        ),
    );
    let translator = tree.script(
        "translator",
        &format!(
            "echo $$ > {m}/translator-pid\ncat > /dev/null\nexec sleep 60",
            m = marks.display()
        ),
    );
    (nexus, translator)
}

fn nothing_made(tree: &Tree) -> bool {
    !tree.path().join("state").exists()
        && !tree.path().join("run").exists()
        && !tree.marks().join("nexus-started").exists()
        && !tree.marks().join("translator-pid").exists()
}

#[test]
fn every_invalid_input_is_refused_before_the_place_is_prepared_or_a_child_started() {
    let tree = Tree::new();
    let (nexus, translator) = children(&tree, false);
    let good = deployment(nexus, translator);
    let place_text = shared_translator::encode(&place(&tree));

    let mut cases: Vec<(String, String)> = vec![
        (place_text.clone(), "not a deployment".into()),
        ("not a place".into(), shared_translator::encode(&good)),
    ];
    let mut unsafe_socket = good.clone();
    unsafe_socket.ordinary_socket_name = SocketName("../escape.sock".into());
    cases.push((
        place_text.clone(),
        shared_translator::encode(&unsafe_socket),
    ));
    let mut unsafe_file = good.clone();
    unsafe_file.memory_file_name = MemoryFileName("..".into());
    cases.push((place_text.clone(), shared_translator::encode(&unsafe_file)));
    let mut sema_file = good.clone();
    sema_file.memory_file_name = MemoryFileName("memory.sema".into());
    cases.push((place_text.clone(), shared_translator::encode(&sema_file)));
    for name in ["x/", "x/.", ".", "..", "", "/abs", "a/b", "./x"] {
        let mut fourth = good.clone();
        fourth.meta_translator_socket_name = SocketName(name.into());
        cases.push((place_text.clone(), shared_translator::encode(&fourth)));
        let mut third = good.clone();
        third.ordinary_translator_socket_name = SocketName(name.into());
        cases.push((place_text.clone(), shared_translator::encode(&third)));
    }
    let mut other_owner = good.clone();
    other_owner.owner = public::Owner::Login(UserId(user() + 1));
    cases.push((place_text.clone(), shared_translator::encode(&other_owner)));
    let mut no_admission = good.clone();
    no_admission.admission = meta::Admission {
        byte_limit: meta::ByteLimit(0),
        timeout: meta::Timeout(1_000),
    };
    cases.push((place_text.clone(), shared_translator::encode(&no_admission)));
    let mut unsafe_place = place(&tree);
    unsafe_place.place_name = PlaceName("../belief".into());
    cases.push((
        shared_translator::encode(&unsafe_place),
        shared_translator::encode(&good),
    ));

    for (place_text, deployment_text) in &cases {
        let output = launch(place_text, deployment_text);
        assert!(!output.status.success(), "{place_text} {deployment_text}");
        assert!(
            nothing_made(&tree),
            "no directory and no child for: {deployment_text}"
        );
    }
}

#[test]
fn the_place_is_prepared_before_either_child_starts_and_a_failing_nexus_ends_the_launcher_without_a_translator_left(
) {
    let tree = Tree::new();
    let (nexus, translator) = children(&tree, true);
    let output = launch_values(&place(&tree), &deployment(nexus, translator));
    assert!(
        !output.status.success(),
        "the launcher never reports a successful startup here"
    );
    let marks = tree.marks();
    assert!(
        marks.join("saw-state").exists() && marks.join("saw-runtime").exists(),
        "prepared before the Nexus child ran"
    );
    let pid: i32 = std::fs::read_to_string(marks.join("translator-pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(
        !Path::new(&format!("/proc/{pid}")).exists(),
        "the translator child {pid} was stopped and reaped"
    );
}

#[test]
fn legacy_state_is_moved_intact_before_the_children_start() {
    let tree = Tree::new();
    std::fs::create_dir_all(tree.legacy()).unwrap();
    std::fs::write(tree.legacy().join("memory"), b"legacy store bytes\0").unwrap();
    let (nexus, translator) = children(&tree, true);
    let output = launch_values(&place(&tree), &deployment(nexus, translator));
    assert!(!output.status.success());
    assert!(!tree.legacy().exists());
    assert_eq!(
        std::fs::read(tree.marks().join("saw-memory")).unwrap(),
        b"legacy store bytes\0",
        "seen by the Nexus child at its start"
    );
    assert_eq!(
        std::fs::read(tree.state().join("memory")).unwrap(),
        b"legacy store bytes\0"
    );
}

#[test]
fn both_directories_refuse_the_launcher_before_any_child_and_change_nothing() {
    let tree = Tree::new();
    std::fs::create_dir_all(tree.legacy()).unwrap();
    std::fs::create_dir_all(tree.state()).unwrap();
    std::fs::write(tree.legacy().join("memory"), b"legacy").unwrap();
    std::fs::write(tree.state().join("memory"), b"current").unwrap();
    let (nexus, translator) = children(&tree, false);
    let output = launch_values(&place(&tree), &deployment(nexus, translator));
    assert!(!output.status.success());
    assert!(
        !tree.marks().join("nexus-started").exists()
            && !tree.marks().join("translator-pid").exists()
    );
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
fn an_explicit_place_is_created_only_and_keeps_its_decoys() {
    let tree = Tree::new();
    let state = tree.path().join("custom-state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("decoy"), b"decoy").unwrap();
    std::fs::create_dir_all(tree.legacy()).unwrap();
    std::fs::write(tree.legacy().join("memory"), b"unrelated").unwrap();
    let explicit = Place {
        place_name: PlaceName("belief".into()),
        runtime: Runtime(
            tree.path()
                .join("custom-run")
                .to_string_lossy()
                .into_owned(),
        ),
        state: State(state.to_string_lossy().into_owned()),
    };
    let (nexus, translator) = children(&tree, true);
    let output = launch_values(&explicit, &deployment(nexus, translator));
    assert!(!output.status.success());
    assert!(tree.path().join("custom-run").is_dir());
    assert_eq!(std::fs::read(state.join("decoy")).unwrap(), b"decoy");
    assert_eq!(
        std::fs::read(tree.legacy().join("memory")).unwrap(),
        b"unrelated"
    );
}
