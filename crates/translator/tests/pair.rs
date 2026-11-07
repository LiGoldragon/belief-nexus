//! The real launcher with the real Nexus and the real translator, in an isolated temporary tree:
//! the sockets and the store appear, one meta call answers Configured, and ending one child ends
//! the launcher and reaps the other. No provider, model, service or production store is used.

use belief_nexus_translator::{
    decode_meta_response_text, encode_meta_query, text_request, Deployment, MemoryFileName,
    NexusExecutable, SocketName, TranslatorExecutable,
};
use meta_signal_belief_nexus as meta;
use shared_translator::placement::{Place, PlaceName, Runtime, State};
use shared_translator::{FrameCapacity, UserId};
use signal_belief_nexus as public;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn sibling(name: &str) -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    let directory = executable.parent().unwrap().parent().unwrap();
    let path = directory.join(name);
    assert!(
        path.exists(),
        "the workspace binary {name} is built beside the test binaries: {}",
        path.display()
    );
    path
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

fn children_of(parent: u32) -> Vec<(u32, String)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir("/proc").unwrap().flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        let after = stat
            .rsplit_once(')')
            .map(|(_, rest)| rest.to_owned())
            .unwrap_or_default();
        let fields: Vec<&str> = after.split_whitespace().collect();
        if fields.get(1).and_then(|field| field.parse::<u32>().ok()) == Some(parent) {
            let command = std::fs::read(entry.path().join("cmdline")).unwrap_or_default();
            found.push((pid, String::from_utf8_lossy(&command).replace('\0', " ")));
        }
    }
    found
}

fn alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[test]
fn the_real_launcher_nexus_and_translator_answer_one_configure_and_reap_together() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let runtime = root.join("run");
    let state = root.join("state");
    let user = i64::from(nix::unistd::Uid::effective().as_raw());
    let place = Place {
        place_name: PlaceName("pair".into()),
        runtime: Runtime(runtime.to_string_lossy().into_owned()),
        state: State(state.to_string_lossy().into_owned()),
    };
    let deployment = Deployment {
        nexus_executable: NexusExecutable(
            sibling("belief-nexus-daemon")
                .to_string_lossy()
                .into_owned(),
        ),
        translator_executable: TranslatorExecutable(env!("CARGO_BIN_EXE_belief-translator").into()),
        ordinary_socket_name: SocketName("ordinary.sock".into()),
        meta_socket_name: SocketName("meta.sock".into()),
        ordinary_translator_socket_name: SocketName("translator.sock".into()),
        meta_translator_socket_name: SocketName("meta-translator.sock".into()),
        memory_file_name: MemoryFileName("memory".into()),
        admission: meta::Admission {
            byte_limit: meta::ByteLimit(1_000_000),
            timeout: meta::Timeout(5_000),
        },
        owner: public::Owner::Login(UserId(user)),
        set_vector: vec![],
    };
    let mut launcher: Child = Command::new(env!("CARGO_BIN_EXE_belief-pair"))
        .args([
            shared_translator::encode(&place),
            shared_translator::encode(&deployment),
        ])
        .env("HOME", root)
        .env("XDG_RUNTIME_DIR", root.join("xdg-run"))
        .env("XDG_STATE_HOME", root.join("xdg-state"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let expected = [
        runtime.join("ordinary.sock"),
        runtime.join("meta.sock"),
        runtime.join("translator.sock"),
        runtime.join("meta-translator.sock"),
        state.join("memory"),
    ];
    until("the sockets and the store", || {
        expected.iter().all(|path| path.exists())
    });

    // The launcher configures revisions 0, 1 and 2 itself (three sets, none from the deployment);
    // a conditional call at revision 3 is refused as stale until it has finished, then answers.
    let query = meta::Query::Configure(meta::ConfigureRequest {
        configuration_revision_option: Some(meta::ConfigurationRevision(3)),
        set: meta::Set::RepositoryPolling(meta::Timeout(25)),
    });
    let mut answered = None;
    until("a real Configured answer", || {
        let text = text_request(
            &runtime.join("meta-translator.sock"),
            &encode_meta_query(&query),
            FrameCapacity::from(signal::MAXIMUM_SIGNAL_BYTES),
        )
        .expect("a real meta call through the real translator to the real Nexus");
        let response = decode_meta_response_text(&text).expect("a meta response");
        let configured = matches!(response, meta::Response::Configured(_));
        if configured {
            answered = Some(response);
        }
        configured
    });
    assert!(matches!(answered, Some(meta::Response::Configured(_))));

    let launcher_pid = launcher.id();
    let pair = children_of(launcher_pid);
    assert_eq!(
        pair.len(),
        2,
        "the launcher has exactly its two children: {pair:?}"
    );
    let nexus = pair
        .iter()
        .find(|(_, command)| command.contains("belief-nexus-daemon"))
        .expect("the Nexus child")
        .0;
    let translator = pair
        .iter()
        .find(|(_, command)| command.contains("belief-translator"))
        .expect("the translator child")
        .0;
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(i32::try_from(nexus).unwrap()),
        nix::sys::signal::Signal::SIGKILL,
    )
    .unwrap();

    until("the launcher to end", || {
        launcher.try_wait().unwrap().is_some()
    });
    let status = launcher.wait().unwrap();
    assert!(
        !status.success(),
        "the launcher never reports a successful startup: {status:?}"
    );
    until("both children to be gone", || {
        !alive(nexus) && !alive(translator)
    });
    assert!(children_of(launcher_pid).is_empty());
}
