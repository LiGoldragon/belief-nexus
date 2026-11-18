//! The real pair as a place starts it, in an isolated temporary tree: the shared pair launcher
//! starts the built Nexus and the built translator with its deployment, and the CLIs take their
//! datom alone, their place read from `nexus/belief.datom` under the configuration home.
//!
//! The same Inputs are answered with the same Outputs: the deployment's twenty settings and the
//! paired Nexus's two sockets are applied first, so the first Configure of the place is answered
//! at revision 23, Ready. A one-answer exchange prints its answer and ends; a Subscribe prints its
//! Opened frame and then each Changed frame; a refused Subscribe prints its refusal and ends.
//! The state survives a restart of the pair, whose deployment the Nexus already holds.

use meta_signal_belief_nexus as meta;
use signal_belief_nexus as public;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use translator_serving::{Budgeting, ValidTextBudget};

const PLACE: &str = "pairtest";

fn nexus_daemon() -> PathBuf {
    let path = PathBuf::from(
        std::env::var_os("BELIEF_NEXUS_DAEMON")
            .expect("BELIEF_NEXUS_DAEMON names the built belief-nexus-daemon executable"),
    );
    assert!(path.exists(), "BELIEF_NEXUS_DAEMON names an existing executable: {}", path.display());
    path
}

/// The Git on the test's path, as the deployment names it.
fn git() -> PathBuf {
    std::env::var_os("PATH")
        .and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|directory| directory.join("git"))
                .find(|candidate| candidate.is_file())
        })
        .expect("git is on the path")
}

fn until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for {what}");
}

struct Tree {
    root: tempfile::TempDir,
}

impl Tree {
    fn new() -> Self {
        let tree = Self { root: tempfile::tempdir().unwrap() };
        std::fs::create_dir_all(tree.root.path().join("config/nexus")).unwrap();
        std::fs::write(tree.root.path().join("config/nexus/belief.datom"), format!("{{ {PLACE} }}")).unwrap();
        tree
    }

    fn path(&self) -> &Path {
        self.root.path()
    }

    fn runtime(&self) -> PathBuf {
        self.path().join(format!("run/nexus/{PLACE}"))
    }

    fn state(&self) -> PathBuf {
        self.path().join("state")
    }

    /// The translator executable the launcher starts: the built translator, given the deployment.
    fn deployed_translator(&self) -> PathBuf {
        let deployment = include_str!("../deployment.datom").replace("@git@", &git().to_string_lossy());
        let script = self.path().join("belief-translator-deployed");
        std::fs::write(
            &script,
            format!("#!/bin/sh\nexec '{}' '{}'\n", env!("CARGO_BIN_EXE_belief-translator"), deployment),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    fn launch(&self) -> Child {
        let place = format!("{{ {PLACE} «{}» «{}» }}", self.runtime().display(), self.state().display());
        let child = Command::new(env!("CARGO_BIN_EXE_belief-pair"))
            .arg(place)
            .arg(nexus_daemon())
            .arg(self.deployed_translator())
            .args(["70000", "8388608", "25", "100000", "64"])
            .spawn()
            .unwrap();
        for socket in ["belief.sock", "belief-meta.sock", "belief-translator.sock", "belief-meta-translator.sock"] {
            let path = self.runtime().join(socket);
            until(&format!("{socket} to appear"), || path.exists());
        }
        child
    }

    fn cli(&self, executable: &str) -> Command {
        let mut command = Command::new(executable);
        command
            .env("XDG_CONFIG_HOME", self.path().join("config"))
            .env("XDG_RUNTIME_DIR", self.path().join("run"));
        command
    }

    /// Every answer a CLI printed, after it exited successfully.
    fn answers<T: datom_codec::Composing>(&self, executable: &str, datom: &str) -> Vec<T> {
        let output: Output = self.cli(executable).arg(datom).output().unwrap();
        assert!(
            output.status.success(),
            "{executable} {datom} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let printed = String::from_utf8(output.stdout).unwrap();
        eprintln!("{executable} '{datom}' =>\n{printed}");
        let mut lines = printed.lines().map(|line| Ok::<_, std::io::Error>(line.to_owned()));
        std::iter::from_fn(|| next_answer(&mut lines)).collect()
    }

    fn meta(&self, datom: &str) -> meta::Output {
        let mut answers = self.answers(env!("CARGO_BIN_EXE_belief-nexus-meta"), datom);
        assert_eq!(answers.len(), 1, "one meta answer: {answers:?}");
        answers.remove(0)
    }

    fn public(&self, datom: &str) -> Vec<public::Output> {
        self.answers(env!("CARGO_BIN_EXE_belief-nexus"), datom)
    }
}

fn decoded<T: datom_codec::Composing>(text: &str) -> Option<T> {
    shared_translator::decode::<T>(text, &mut ValidTextBudget::MAXIMUM.budget()).ok()
}

/// The next answer a CLI printed: its lines, read until they are one whole answer. An answer's
/// text may span lines.
fn next_answer<T: datom_codec::Composing>(
    lines: &mut impl Iterator<Item = std::io::Result<String>>,
) -> Option<T> {
    let mut text = String::new();
    for line in lines {
        text.push_str(&line.unwrap());
        text.push('\n');
        if let Some(answer) = decoded(&text) {
            return Some(answer);
        }
    }
    assert!(text.trim().is_empty(), "printed text that is no answer: {text}");
    None
}

fn stop(mut launcher: Child) {
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(i32::try_from(launcher.id()).unwrap()),
        nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    launcher.wait().unwrap();
}

fn configured(set: meta::Set, revision: i64) -> meta::Output {
    meta::Output::Configured(meta::ConfiguredState {
        set,
        keeper: meta::Keeper::Nexus,
        configuration_revision: meta::ConfigurationRevision(revision),
        readiness: meta::Readiness::Ready,
    })
}

fn repository_fixture(repository: &Path) {
    std::fs::create_dir_all(repository.join("spirit")).unwrap();
    std::fs::write(repository.join("spirit/core.md"), "The bundle witness is PAIR_WITNESS.\n").unwrap();
    for args in [
        ["init", "-b", "main"].as_slice(),
        ["config", "user.email", "belief@example.test"].as_slice(),
        ["config", "user.name", "Belief Test"].as_slice(),
        ["add", "."].as_slice(),
        ["commit", "-m", "fixture"].as_slice(),
    ] {
        assert!(Command::new(git()).arg("-C").arg(repository).args(args).status().unwrap().success());
    }
}

fn furnish(workspace: &Path) -> String {
    std::fs::create_dir_all(workspace).unwrap();
    format!("Furnish.{{ «{}» Codex [ {{ Spirit core }} ] }}", workspace.display())
}

#[test]
fn the_pair_answers_as_before_streams_its_frames_and_ends_a_refused_subscribe() {
    let tree = Tree::new();
    let launcher = tree.launch();

    assert_eq!(
        tree.meta("Configure.{ None RepositoryPolling.211 }"),
        configured(meta::Set::RepositoryPolling(meta::Timeout(211)), 23),
        "the deployment's twenty settings and the paired Nexus's two sockets come first"
    );
    assert_eq!(
        tree.public("Resolve.[]"),
        vec![public::Output::Resolved(public::Resolution {
            found: public::Found(Vec::new()),
            missing: public::Missing(Vec::new()),
        })],
        "a one-answer exchange prints its answer and ends"
    );
    let uid = nix::unistd::getuid().as_raw();
    let peer = format!("Configure.{{ None OrdinaryPeers.[ {{ flow {{ {uid} {} }} }} ] }}", std::process::id());
    assert_eq!(
        tree.meta(&peer),
        configured(
            meta::Set::OrdinaryPeers(public::PeerList(vec![shared_translator::PeerBinding {
                peer_id: shared_translator::PeerId("flow".into()),
                peer_identity: shared_translator::PeerIdentity {
                    user_id: shared_translator::UserId(i64::from(uid)),
                    process_id: shared_translator::ProcessId(i64::from(std::process::id())),
                },
            }])),
            25,
        )
    );

    let repository = tree.path().join("beliefs");
    repository_fixture(&repository);
    let registered = tree.public(&format!("Register.«{}»", repository.display()));
    assert!(matches!(registered.as_slice(), [public::Output::Registered(_)]), "{registered:?}");
    let first = tree.path().join("first");
    let furnished = tree.public(&furnish(&first));
    assert!(matches!(furnished.as_slice(), [public::Output::Furnished(_)]), "{furnished:?}");

    let mut subscriber = tree
        .cli(env!("CARGO_BIN_EXE_belief-nexus"))
        .arg("Subscribe")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut frames = BufReader::new(subscriber.stdout.take().unwrap()).lines();
    let mut frame = || -> public::Output {
        let answer = next_answer(&mut frames).expect("a frame");
        eprintln!("belief-nexus 'Subscribe' => {answer:?}");
        answer
    };
    let public::Output::Subscribed(signal::Frame::Opened(public::Workspaces(opened))) = frame() else {
        panic!("the first frame is Opened")
    };
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].workspace.0 .0, first.to_string_lossy());
    let second = tree.path().join("second");
    let furnished = tree.public(&furnish(&second));
    assert!(matches!(furnished.as_slice(), [public::Output::Furnished(_)]), "{furnished:?}");
    let public::Output::Subscribed(signal::Frame::Changed(public::Furnishing::Furnished(changed))) = frame() else {
        panic!("the next frame is the furnishing")
    };
    assert_eq!(changed.workspace.0 .0, second.to_string_lossy());

    assert_eq!(
        tree.meta("Configure.{ None SubscriberCapacity.1 }"),
        configured(meta::Set::SubscriberCapacity(1), 26)
    );
    assert_eq!(
        tree.public("Subscribe"),
        vec![public::Output::Refused(public::Refusal::SubscriberCapacityReached(1))],
        "a refused Subscribe prints its refusal and ends"
    );
    subscriber.kill().unwrap();
    subscriber.wait().unwrap();
    stop(launcher);
    for socket in ["belief.sock", "belief-meta.sock", "belief-translator.sock", "belief-meta-translator.sock"] {
        let path = tree.runtime().join(socket);
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }
    }
    assert!(tree.state().join("belief.memory").exists(), "the Nexus keeps belief.memory in the place");

    let restarted = tree.launch();
    assert_eq!(
        tree.meta("Configure.{ None RepositoryPolling.212 }"),
        configured(meta::Set::RepositoryPolling(meta::Timeout(212)), 27),
        "a restarted pair applies none of a deployment the Nexus already holds"
    );
    let resolved = tree.public("Resolve.[ { Spirit core } ]");
    let [public::Output::Resolved(resolution)] = resolved.as_slice() else {
        panic!("the restarted pair resolves: {resolved:?}")
    };
    assert_eq!(resolution.found.0.len(), 1, "the registered repository survived the restart");
    stop(restarted);
}

#[test]
fn a_cli_with_no_configuration_sends_nothing_and_names_the_file() {
    let tree = Tree::new();
    std::fs::remove_file(tree.path().join("config/nexus/belief.datom")).unwrap();
    let output = tree.cli(env!("CARGO_BIN_EXE_belief-nexus")).arg("Resolve.[]").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let refusal = String::from_utf8_lossy(&output.stderr);
    assert!(refusal.starts_with("Missing."), "{refusal}");
    assert!(refusal.contains("nexus/belief.datom"), "{refusal}");
}
