use meta_signal_belief_nexus as meta;
use signal::{
    ByteViewable, FrameBody, FrameReading, FrameWriting, Restorable, Signal, Signalizable,
};
use signal_belief_nexus as wire;
use std::os::unix::net::{UnixListener, UnixStream};
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
    let authorized = std::env::var_os("BELIEF_DIRECT_PEER_AUTHORIZED").is_some();
    if authorized {
        let repository = wire::Repository(wire::RepositoryPath(std::env::var("BELIEF_DIRECT_REPOSITORY").unwrap()));
        let response: wire::Response = exchange(Path::new(&socket), &wire::Call { caller: caller.clone(), query: wire::Query::Register(repository) });
        assert!(matches!(response, wire::Response::Registered(_)), "configured direct Register failed: {response:?}");
        let workspace = wire::Workspace(wire::WorkspacePath(std::env::var("BELIEF_DIRECT_WORKSPACE").unwrap()));
        let response: wire::Response = exchange(Path::new(&socket), &wire::Call { caller: caller.clone(), query: wire::Query::Furnish(wire::FurnishRequest {
            workspace: workspace.clone(), harness: wire::Harness::Codex,
            module_name_vector: vec![wire::ModuleName { kind: wire::Kind::Spirit, subject: wire::Subject("core".into()) }],
        }) });
        let wire::Response::Furnished(furnished) = response else { panic!("configured direct Furnish failed: {response:?}") };
        assert_eq!(furnished.workspace, workspace);
        assert!(Path::new(&(furnished.furnished_paths.system_prompt_path.0)).exists());
        assert!(Path::new(&(furnished.furnished_paths.skills_path.0)).exists());
        let claude_workspace = wire::Workspace(wire::WorkspacePath(std::env::var("BELIEF_DIRECT_CLAUDE_WORKSPACE").unwrap()));
        let response: wire::Response = exchange(Path::new(&socket), &wire::Call { caller, query: wire::Query::Furnish(wire::FurnishRequest {
            workspace: claude_workspace.clone(), harness: wire::Harness::Claude,
            module_name_vector: vec![
                wire::ModuleName { kind: wire::Kind::Spirit, subject: wire::Subject("core".into()) },
                wire::ModuleName { kind: wire::Kind::Knowledge, subject: wire::Subject("witness".into()) },
            ],
        }) });
        let wire::Response::Furnished(furnished) = response else { panic!("configured direct Claude Furnish failed: {response:?}") };
        assert_eq!(furnished.workspace, claude_workspace);
        assert!(Path::new(&(furnished.furnished_paths.system_prompt_path.0)).exists());
        assert!(Path::new(&(furnished.furnished_paths.skills_path.0)).exists());
    } else {
        let workspace = wire::Workspace(wire::WorkspacePath(std::env::var("BELIEF_DIRECT_WORKSPACE").unwrap()));
        let response: wire::Response = exchange(Path::new(&socket), &wire::Call { caller, query: wire::Query::Furnish(wire::FurnishRequest {
            workspace, harness: wire::Harness::Codex,
            module_name_vector: vec![wire::ModuleName { kind: wire::Kind::Spirit, subject: wire::Subject("core".into()) }],
        }) });
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
    let emission_socket = directory.path().join("translator.sock");
    let emission_listener = UnixListener::bind(&emission_socket).unwrap();
    emission_listener.set_nonblocking(true).unwrap();
    let emission_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let emission_running = emission_stop.clone();
    let emission_thread = thread::spawn(move || {
        let executor = emission_belief_nexus::FileEmission::new(std::path::PathBuf::from("git")).unwrap();
        while !emission_running.load(std::sync::atomic::Ordering::Acquire) {
            match emission_listener.accept() {
                Ok((mut stream, _)) => {
                    let body = stream.read_frame(signal::FrameCapacity::default()).unwrap();
                    let attempt = Signal::<emission_belief_nexus::EmissionAttempt>::from(Vec::<u8>::from(body)).restore().unwrap();
                    let outcome = emission_belief_nexus::EmissionAttemptOutcome { configuration_revision: attempt.configuration_revision, response: executor.execute(attempt.query) };
                    let archived = outcome.signalize().unwrap();
                    stream.write_frame(&FrameBody::from(archived.bytes().to_vec()), signal::FrameCapacity::default()).unwrap();
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(2)),
                Err(error) => panic!("emission listener: {error}"),
            }
        }
    });
    let bootstrap = meta::NexusBootstrap {
        ordinary_socket: meta::OrdinarySocket(meta::SocketPath(
            ordinary.to_string_lossy().into_owned(),
        )),
        meta_socket: meta::MetaSocket(meta::SocketPath(privileged.to_string_lossy().into_owned())),
        emission_translator_socket: meta::EmissionTranslatorSocket(meta::SocketPath(emission_socket.to_string_lossy().into_owned())),
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
        bootstrap_owner: meta::BootstrapOwner(wire::Owner::Login(caller.user_id.clone())),
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
    let repository = directory.path().join("beliefs");
    let workspace = directory.path().join("workspace");
    let claude_workspace = directory.path().join("claude-workspace");
    std::fs::create_dir_all(repository.join("spirit")).unwrap();
    std::fs::create_dir_all(repository.join("knowledge")).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&claude_workspace).unwrap();
    std::fs::write(repository.join("spirit/core.md"), "The exact Claude bundle witness is CLAUDE_BUNDLE_WITNESS.\n").unwrap();
    std::fs::write(repository.join("knowledge/witness.md"), "---\nname: knowledge-witness\ndescription: Use for the Belief Nexus Claude proof.\n---\nState the exact skill witness CLAUDE_SKILL_WITNESS.\n").unwrap();
    for args in [["init", "-b", "main"].as_slice(), ["config", "user.email", "belief@example.test"].as_slice(), ["config", "user.name", "Belief Test"].as_slice(), ["add", "."].as_slice(), ["commit", "-m", "fixture"].as_slice()] {
        assert!(Command::new("git").arg("-C").arg(&repository).args(args).status().unwrap().success());
    }
    let current = std::env::current_exe().unwrap();
    let authorized_gate = directory.path().join("authorized.gate");
    let mut authorized = Command::new(&current)
        .args(["--exact", "direct_peer_child", "--nocapture"])
        .env("BELIEF_DIRECT_PEER_SOCKET", &ordinary)
        .env("BELIEF_DIRECT_PEER_GATE", &authorized_gate)
        .env("BELIEF_DIRECT_PEER_AUTHORIZED", "1")
        .env("BELIEF_DIRECT_REPOSITORY", &repository)
        .env("BELIEF_DIRECT_WORKSPACE", &workspace)
        .env("BELIEF_DIRECT_CLAUDE_WORKSPACE", &claude_workspace)
        .spawn().unwrap();
    let binding = translator::PeerBinding {
        peer_id: translator::PeerId("flow-peer".into()),
        peer_identity: translator::PeerIdentity {
            user_id: translator::UserId(i64::from(nix::unistd::getuid().as_raw())),
            process_id: translator::ProcessId(i64::from(authorized.id())),
        },
    };
    let layout = meta::Layout {
        owned_root: meta::OwnedRoot(wire::RelativePath(".belief".into())), generations_directory: meta::GenerationsDirectory(wire::RelativePath("generations".into())),
        current_link: meta::CurrentLink(wire::RelativePath("current".into())), settings_file: meta::SettingsFile(wire::RelativePath(".codex/config.toml".into())),
        instructions_setting: meta::InstructionsSetting("model_instructions_file".into()), instructions_file: meta::InstructionsFile(wire::RelativePath("instructions.md".into())),
        skills_directory: meta::SkillsDirectory(wire::RelativePath(".agents/skills".into())),
    };
    let sets = vec![
        meta::Set::RepositoryPolling(meta::Timeout(20)), meta::Set::SubscriberCapacity(2),
        meta::Set::OrdinaryPeers(wire::PeerList(vec![binding])), meta::Set::MetaPeers(wire::PeerList(vec![identity.clone()])),
        meta::Set::Layout(meta::HarnessLayout { harness: wire::Harness::Codex, layout }),
        meta::Set::Layout(meta::HarnessLayout { harness: wire::Harness::Claude, layout: meta::Layout {
            owned_root: meta::OwnedRoot(wire::RelativePath(".belief".into())), generations_directory: meta::GenerationsDirectory(wire::RelativePath("generations".into())),
            current_link: meta::CurrentLink(wire::RelativePath("current".into())), settings_file: meta::SettingsFile(wire::RelativePath("CLAUDE.md".into())),
            instructions_setting: meta::InstructionsSetting("unused-for-claude".into()), instructions_file: meta::InstructionsFile(wire::RelativePath("CLAUDE.md".into())),
            skills_directory: meta::SkillsDirectory(wire::RelativePath(".claude/skills".into())),
        } }),
        meta::Set::Placement(meta::PlacementDefault { kind: wire::Kind::Spirit, placement: wire::Placement::SystemPrompt }),
        meta::Set::Placement(meta::PlacementDefault { kind: wire::Kind::Knowledge, placement: wire::Placement::Skills }),
        meta::Set::NexusFrameCapacity(meta::ByteLimit(1_000_000)), meta::Set::NexusTransportTimeout(meta::Timeout(1_000)),
        meta::Set::NexusSocket(meta::SocketPath("unused".into())), meta::Set::MetaNexusSocket(meta::SocketPath("unused-meta".into())),
        meta::Set::CallTimeout(meta::Timeout(1_000)), meta::Set::OutputLimit(meta::ByteLimit(1_000_000)),
        meta::Set::TextBudget(meta::Budget { node_limit: meta::NodeLimit(10_000), depth_limit: meta::DepthLimit(100) }),
        meta::Set::TranslatorFrameCapacity(meta::ByteLimit(1_000_000)), meta::Set::TranslatorTransportTimeout(meta::Timeout(1_000)),
        meta::Set::AllowedNexusPeers(wire::PeerList(vec![identity.clone()])), meta::Set::GitExecutable(wire::ExecutablePath("git".into())),
    ];
    for (revision, set) in sets.into_iter().enumerate() {
        let configured: meta::MetaTransportOutput = exchange(&privileged, &meta::MetaTransportInput::Call(meta::MetaCall { caller: caller.clone(), query: meta::Query::Configure(meta::ConfigureRequest { configuration_revision_option: Some(meta::ConfigurationRevision(revision as i64)), set }) }));
        assert!(matches!(configured, meta::MetaTransportOutput::Answered(meta::Response::Configured(_))), "configuration revision {revision} failed: {configured:?}");
    }
    std::fs::write(&authorized_gate, b"go").unwrap();
    assert!(authorized.wait().unwrap().success(), "configured direct peer process failed");
    assert_eq!(std::fs::read_link(claude_workspace.join("CLAUDE.md")).unwrap(), claude_workspace.join(".belief/current/CLAUDE.md"));
    assert_eq!(std::fs::read_link(claude_workspace.join(".claude/skills")).unwrap(), claude_workspace.join(".belief/current/skills"));
    if let Some(claude) = std::env::var_os("BELIEF_LIVE_CLAUDE") {
        let output = Command::new(claude).current_dir(&claude_workspace)
            .args(["--print", "--dangerously-skip-permissions", "State the exact bundle witness. Invoke $knowledge-witness and state its exact skill witness."])
            .output().unwrap();
        assert!(output.status.success(), "Claude failed: {}", String::from_utf8_lossy(&output.stderr));
        let witness = String::from_utf8(output.stdout).unwrap();
        assert!(witness.contains("CLAUDE_BUNDLE_WITNESS"), "Claude omitted bundle witness: {witness}");
        assert!(witness.contains("CLAUDE_SKILL_WITNESS"), "Claude omitted skill witness: {witness}");
    }

    let refused_gate = directory.path().join("refused.gate");
    let mut refused = Command::new(&current)
        .args(["--exact", "direct_peer_child", "--nocapture"])
        .env("BELIEF_DIRECT_PEER_SOCKET", &ordinary)
        .env("BELIEF_DIRECT_PEER_GATE", &refused_gate)
        .env("BELIEF_DIRECT_WORKSPACE", &workspace)
        .spawn().unwrap();
    std::fs::write(&refused_gate, b"go").unwrap();
    assert!(refused.wait().unwrap().success(), "unconfigured direct peer process did not receive typed refusal");
    child.kill().unwrap();
    child.wait().unwrap();
    emission_stop.store(true, std::sync::atomic::Ordering::Release);
    emission_thread.join().unwrap();
}
