//! The real Nexus process, paired with this test process as its translator: its meta and public
//! sockets speak the exchange layer (greet, Open, Answer, End), a one-answer exchange is answered
//! once and ended, a Subscribe is answered with the frames of a stream and a refused one is
//! answered once and ended, direct peers are admitted by their configured binding, and the state
//! survives a restart.

use belief_nexus_wire::{meta_call, public_call, BeliefNexus};
use meta_signal_belief_nexus as meta;
use nexus_exchange::lane::{Frame, Kind, Lane, LaneReading, LaneWriting};
use nexus_exchange::ExchangeError;
use nexus_querying::{ExchangeFollowing, Patience, Querying, QueryingConnection};
use nexus_serving::{InstanceId, NexusBootstrap};
use signal::{ByteViewable, Delivery, FrameBody, FrameWriting, Restorable, Signal, Signalizable};
use signal_belief_nexus as wire;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

type SocketPaths = nexus_exchange::SocketPaths<BeliefNexus>;

fn capacity() -> signal::FrameCapacity {
    signal::FrameCapacity::from(signal::MAXIMUM_SIGNAL_BYTES)
}

fn own_caller() -> translator::Caller {
    translator::Caller {
        user_id: translator::UserId(i64::from(nix::unistd::getuid().as_raw())),
        process_id: translator::ProcessId(i64::from(std::process::id())),
    }
}

/// Every answer of one public exchange, read until the Nexus ends it Completed.
fn public_answers(socket: &Path, caller: translator::Caller, query: wire::Input) -> Vec<wire::Output> {
    let stream = UnixStream::connect(socket).unwrap();
    let mut connection = QueryingConnection::<public_call::Input, public_call::Output>::greeted(
        stream,
        Patience::Until(Instant::now() + Duration::from_secs(30)),
        capacity(),
    )
    .unwrap();
    let exchange = connection
        .open(public_call::Input::Call(public_call::Call_Data { caller, query }))
        .unwrap();
    let mut answers = Vec::new();
    connection
        .follow::<ExchangeError>(exchange, |response| {
            match response {
                public_call::Output::Answered(output) => answers.push(output),
                public_call::Output::Failed(message) => panic!("the Nexus failed: {message}"),
            }
            Ok(())
        })
        .expect("the exchange ends Completed");
    answers
}

/// One public exchange: greet, open the call, read its one answer and its End.
fn public(socket: &Path, caller: translator::Caller, query: wire::Input) -> wire::Output {
    let mut answers = public_answers(socket, caller, query);
    assert_eq!(answers.len(), 1, "a one-answer exchange is answered once: {answers:?}");
    answers.remove(0)
}

/// One meta exchange, as the paired translator: its one answer, then its End.
fn meta_exchange(socket: &Path, input: meta_call::Input) -> meta_call::Output {
    let stream = UnixStream::connect(socket).unwrap();
    let mut connection = QueryingConnection::<meta_call::Input, meta_call::Output>::greeted(
        stream,
        Patience::Until(Instant::now() + Duration::from_secs(30)),
        capacity(),
    )
    .unwrap();
    let exchange = connection.open(input).unwrap();
    let mut answers = Vec::new();
    connection
        .follow::<ExchangeError>(exchange, |response| {
            answers.push(response);
            Ok(())
        })
        .expect("the meta exchange ends Completed");
    assert_eq!(answers.len(), 1, "a meta exchange is answered once: {answers:?}");
    answers.remove(0)
}

fn configure(socket: &Path, revision: i64, set: meta::Set) -> meta::Output {
    match meta_exchange(
        socket,
        meta_call::Input::Call(meta_call::Call_Data {
            caller: own_caller(),
            query: meta::Input::Configure(meta::ConfigureRequest {
                configuration_revision_option: Some(meta::ConfigurationRevision(revision)),
                set,
            }),
        }),
    ) {
        meta_call::Output::Answered(output) => output,
        other => panic!("configuration revision {revision} was not answered: {other:?}"),
    }
}

/// The paired translator's emission lane, answered with the real file emission.
fn serve_emissions(socket: &Path) {
    let listener = UnixListener::bind(socket).unwrap();
    thread::spawn(move || {
        let executor = emission_belief_nexus::FileEmission::new(PathBuf::from("git")).unwrap();
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let request = stream.read_lane_frame(signal::MAXIMUM_SIGNAL_BYTES).unwrap();
            let attempt = Signal::<emission_belief_nexus::EmissionAttempt>::from(request.payload)
                .restore()
                .unwrap();
            let outcome = emission_belief_nexus::EmissionAnswer::Attempted(
                emission_belief_nexus::EmissionAttemptOutcome {
                    configuration_revision: attempt.configuration_revision,
                    output: executor.execute(attempt.input),
                },
            );
            stream
                .write_lane_frame(
                    &Frame {
                        lane: Lane::EmissionSignal,
                        kind: Kind::Answered,
                        payload: outcome.signalize().unwrap().bytes().to_vec(),
                    },
                    signal::MAXIMUM_SIGNAL_BYTES,
                )
                .unwrap();
        }
    });
}

fn place(directory: &Path, name: &str) -> translator::Place {
    translator::Place {
        place_name: translator::PlaceName(name.into()),
        runtime: translator::Runtime(directory.to_string_lossy().into_owned()),
        state: translator::State(directory.to_string_lossy().into_owned()),
    }
}

/// A bootstrap paired with this test process.
fn bootstrap(place: translator::Place) -> NexusBootstrap {
    let caller = own_caller();
    NexusBootstrap {
        place,
        admission: nexus_exchange::Admission {
            frame_deadline_milliseconds: nexus_exchange::FrameDeadlineMilliseconds(5_000),
            frame_capacity_bytes: nexus_exchange::FrameCapacityBytes(1_000_000),
            retry_delay_milliseconds: nexus_exchange::RetryDelayMilliseconds(25),
        },
        peer_identity: translator::PeerIdentity {
            user_id: caller.user_id,
            process_id: caller.process_id,
        },
        instance_id: InstanceId("process".into()),
    }
}

fn spawn_nexus(bootstrap: &NexusBootstrap) -> Child {
    let mut child = Command::new(env!("CARGO_BIN_EXE_belief-nexus-daemon"))
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_frame(
            &FrameBody::from(bootstrap.signalize().unwrap().bytes().to_vec()),
            signal::FrameCapacity::default(),
        )
        .unwrap();
    drop(child.stdin.take());
    child
}

fn wait_for(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(path.exists(), "{} did not appear", path.display());
}

fn layout(settings: &str, instructions_setting: &str, instructions: &str, skills: &str) -> meta::Layout {
    meta::Layout {
        owned_root: meta::OwnedRoot(wire::RelativePath(".belief".into())),
        generations_directory: meta::GenerationsDirectory(wire::RelativePath("generations".into())),
        current_link: meta::CurrentLink(wire::RelativePath("current".into())),
        settings_file: meta::SettingsFile(wire::RelativePath(settings.into())),
        instructions_setting: meta::InstructionsSetting(instructions_setting.into()),
        instructions_file: meta::InstructionsFile(wire::RelativePath(instructions.into())),
        skills_directory: meta::SkillsDirectory(wire::RelativePath(skills.into())),
    }
}

/// Every setting a ready Nexus needs, its ordinary peers as given.
fn ready_sets(ordinary_peers: Vec<translator::PeerBinding>) -> Vec<meta::Set> {
    let caller = own_caller();
    let identity = translator::PeerBinding {
        peer_id: translator::PeerId("process-test".into()),
        peer_identity: translator::PeerIdentity {
            user_id: caller.user_id,
            process_id: caller.process_id,
        },
    };
    vec![
        meta::Set::RepositoryPolling(meta::Timeout(20)),
        meta::Set::SubscriberCapacity(2),
        meta::Set::OrdinaryPeers(wire::PeerList(ordinary_peers)),
        meta::Set::MetaPeers(wire::PeerList(vec![identity.clone()])),
        meta::Set::HarnessLayout(meta::HarnessLayout {
            harness: wire::Harness::Codex,
            layout: layout(".codex/config.toml", "model_instructions_file", "instructions.md", ".agents/skills"),
        }),
        meta::Set::HarnessLayout(meta::HarnessLayout {
            harness: wire::Harness::Claude,
            layout: layout("CLAUDE.md", "unused-for-claude", "CLAUDE.md", ".claude/skills"),
        }),
        meta::Set::PlacementDefault(meta::PlacementDefault {
            kind: wire::Kind::Spirit,
            placement: wire::Placement::SystemPrompt,
        }),
        meta::Set::PlacementDefault(meta::PlacementDefault {
            kind: wire::Kind::Knowledge,
            placement: wire::Placement::Skills,
        }),
        meta::Set::NexusFrameCapacity(meta::ByteLimit(1_000_000)),
        meta::Set::NexusTransportTimeout(meta::Timeout(1_000)),
        meta::Set::NexusSocket(meta::SocketPath("unused".into())),
        meta::Set::MetaNexusSocket(meta::SocketPath("unused-meta".into())),
        meta::Set::CallTimeout(meta::Timeout(1_000)),
        meta::Set::OutputLimit(meta::ByteLimit(1_000_000)),
        meta::Set::TextBudget(meta::Budget {
            node_limit: meta::NodeLimit(10_000),
            depth_limit: meta::DepthLimit(100),
        }),
        meta::Set::TranslatorFrameCapacity(meta::ByteLimit(1_000_000)),
        meta::Set::TranslatorTransportTimeout(meta::Timeout(1_000)),
        meta::Set::AllowedNexusPeers(wire::PeerList(vec![identity])),
        meta::Set::GitExecutable(wire::ExecutablePath("git".into())),
    ]
}

fn repository_fixture(repository: &Path) {
    std::fs::create_dir_all(repository.join("spirit")).unwrap();
    std::fs::create_dir_all(repository.join("knowledge")).unwrap();
    std::fs::write(repository.join("spirit/core.md"), "The exact Claude bundle witness is CLAUDE_BUNDLE_WITNESS.\n").unwrap();
    std::fs::write(repository.join("knowledge/witness.md"), "---\nname: knowledge-witness\ndescription: Use for the Belief Nexus Claude proof.\n---\nState the exact skill witness CLAUDE_SKILL_WITNESS.\n").unwrap();
    for args in [["init", "-b", "main"].as_slice(), ["config", "user.email", "belief@example.test"].as_slice(), ["config", "user.name", "Belief Test"].as_slice(), ["add", "."].as_slice(), ["commit", "-m", "fixture"].as_slice()] {
        assert!(Command::new("git").arg("-C").arg(repository).args(args).status().unwrap().success());
    }
}

fn furnish_request(workspace: &Path, harness: wire::Harness, names: &[(wire::Kind, &str)]) -> wire::Input {
    wire::Input::Furnish(wire::FurnishRequest {
        workspace: wire::Workspace(wire::WorkspacePath(workspace.to_string_lossy().into_owned())),
        harness,
        module_name_vector: names
            .iter()
            .map(|(kind, subject)| wire::ModuleName { kind: kind.clone(), subject: wire::Subject((*subject).into()) })
            .collect(),
    })
}

#[test]
fn direct_peer_child() {
    let Some(socket) = std::env::var_os("BELIEF_DIRECT_PEER_SOCKET") else { return };
    let gate = std::path::PathBuf::from(std::env::var_os("BELIEF_DIRECT_PEER_GATE").unwrap());
    for _ in 0..500 {
        if gate.exists() { break; }
        thread::sleep(Duration::from_millis(2));
    }
    assert!(gate.exists(), "direct peer gate never opened");
    let caller = own_caller();
    let socket = Path::new(&socket);
    let authorized = std::env::var_os("BELIEF_DIRECT_PEER_AUTHORIZED").is_some();
    if authorized {
        let repository = wire::Repository(wire::RepositoryPath(std::env::var("BELIEF_DIRECT_REPOSITORY").unwrap()));
        let response = public(socket, caller.clone(), wire::Input::Register(repository));
        assert!(matches!(response, wire::Output::Registered(_)), "configured direct Register failed: {response:?}");
        let workspace = PathBuf::from(std::env::var("BELIEF_DIRECT_WORKSPACE").unwrap());
        let response = public(socket, caller.clone(), furnish_request(&workspace, wire::Harness::Codex, &[(wire::Kind::Spirit, "core")]));
        let wire::Output::Furnished(furnished) = response else { panic!("configured direct Furnish failed: {response:?}") };
        assert_eq!(furnished.workspace.0 .0, workspace.to_string_lossy());
        assert!(Path::new(&(furnished.furnished_paths.system_prompt_path.0)).exists());
        assert_eq!(PathBuf::from(&furnished.furnished_paths.skills_path.0), workspace.join(".agents/skills"));
        assert!(Path::new(&(furnished.furnished_paths.skills_path.0)).exists());
        let claude_workspace = PathBuf::from(std::env::var("BELIEF_DIRECT_CLAUDE_WORKSPACE").unwrap());
        let response = public(socket, caller, furnish_request(&claude_workspace, wire::Harness::Claude, &[(wire::Kind::Spirit, "core"), (wire::Kind::Knowledge, "witness")]));
        let wire::Output::Furnished(furnished) = response else { panic!("configured direct Claude Furnish failed: {response:?}") };
        assert_eq!(furnished.workspace.0 .0, claude_workspace.to_string_lossy());
        assert!(Path::new(&(furnished.furnished_paths.system_prompt_path.0)).exists());
        assert_eq!(PathBuf::from(&furnished.furnished_paths.skills_path.0), claude_workspace.join(".claude/skills"));
        assert!(Path::new(&(furnished.furnished_paths.skills_path.0)).exists());
    } else {
        let workspace = PathBuf::from(std::env::var("BELIEF_DIRECT_WORKSPACE").unwrap());
        let response = public(socket, caller, furnish_request(&workspace, wire::Harness::Codex, &[(wire::Kind::Spirit, "core")]));
        assert!(matches!(response, wire::Output::Refused(wire::Refusal::NotAuthorized(_))), "unconfigured direct peer was admitted: {response:?}");
    }
}

#[test]
fn nexus_process_serves_authenticated_meta_and_ordinary_signal() {
    let directory = tempfile::tempdir().unwrap();
    let bootstrap = bootstrap(place(directory.path(), "process"));
    let paths = SocketPaths::from(&bootstrap.place);
    serve_emissions(&paths.translator);
    let mut child = spawn_nexus(&bootstrap);
    wait_for(&paths.meta);
    wait_for(&paths.public);
    let repository = directory.path().join("beliefs");
    let workspace = directory.path().join("workspace");
    let claude_workspace = directory.path().join("claude-workspace");
    repository_fixture(&repository);
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&claude_workspace).unwrap();
    let current = std::env::current_exe().unwrap();
    let authorized_gate = directory.path().join("authorized.gate");
    let mut authorized = Command::new(&current)
        .args(["--exact", "direct_peer_child", "--nocapture"])
        .env("BELIEF_DIRECT_PEER_SOCKET", &paths.public)
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
    for (revision, set) in ready_sets(vec![binding]).into_iter().enumerate() {
        let configured = configure(&paths.meta, revision as i64, set);
        assert!(matches!(configured, meta::Output::Configured(_)), "configuration revision {revision} failed: {configured:?}");
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
        .env("BELIEF_DIRECT_PEER_SOCKET", &paths.public)
        .env("BELIEF_DIRECT_PEER_GATE", &refused_gate)
        .env("BELIEF_DIRECT_WORKSPACE", &workspace)
        .spawn().unwrap();
    std::fs::write(&refused_gate, b"go").unwrap();
    assert!(refused.wait().unwrap().success(), "unconfigured direct peer process did not receive typed refusal");
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn every_exchange_is_answered_on_its_own_identifier_and_the_translator_restores_its_settings() {
    let directory = tempfile::tempdir().unwrap();
    let bootstrap = bootstrap(place(directory.path(), "exchange"));
    let paths = SocketPaths::from(&bootstrap.place);
    serve_emissions(&paths.translator);
    let mut child = spawn_nexus(&bootstrap);
    wait_for(&paths.meta);
    wait_for(&paths.public);
    assert!(
        matches!(public(&paths.public, own_caller(), wire::Input::Resolve(Vec::new())), wire::Output::Refused(wire::Refusal::NotReady)),
        "an unconfigured Nexus answers NotReady"
    );
    // A refused Subscribe is answered once and its exchange ended, as every exchange is.
    let refused = public_answers(&paths.public, own_caller(), wire::Input::Subscribe);
    assert_eq!(refused, vec![wire::Output::Refused(wire::Refusal::NotReady)], "a refused Subscribe ends after its refusal");
    for (revision, set) in ready_sets(Vec::new()).into_iter().enumerate() {
        assert!(matches!(configure(&paths.meta, revision as i64, set), meta::Output::Configured(_)));
    }
    let stale = configure(&paths.meta, 0, meta::Set::RepositoryPolling(meta::Timeout(25)));
    assert!(matches!(stale, meta::Output::ConfigureRefused(meta::ConfigurationRefusal::StaleRevision(_))), "{stale:?}");
    let restored = meta_exchange(&paths.meta, meta_call::Input::RestoreTranslatorConfiguration);
    let meta_call::Output::Restored(snapshot) = restored else { panic!("no translator settings restored: {restored:?}") };
    assert_eq!(snapshot.readiness, meta::Readiness::Ready);
    assert_eq!(snapshot.set_vector.len(), 9, "the nine translator-kept settings: {snapshot:?}");

    // Two exchanges open on one connection are each answered on their own identifier.
    let stream = UnixStream::connect(&paths.public).unwrap();
    let mut connection = QueryingConnection::<public_call::Input, public_call::Output>::greeted(
        stream,
        Patience::Until(Instant::now() + Duration::from_secs(30)),
        capacity(),
    )
    .unwrap();
    let call = |query| public_call::Input::Call(public_call::Call_Data { caller: own_caller(), query });
    let first = connection.open(call(wire::Input::Resolve(Vec::new()))).unwrap();
    let second = connection.open(call(wire::Input::Resolve(Vec::new()))).unwrap();
    assert_ne!(first, second);
    let mut answered = Vec::new();
    while answered.len() < 2 {
        match connection.delivery().unwrap() {
            Some(Delivery::Answer(answer)) => {
                assert!(matches!(answer.response, public_call::Output::Answered(wire::Output::Resolved(_))));
                answered.push(answer.exchange);
            }
            Some(_) => {}
            None => panic!("the connection closed before both answers"),
        }
    }
    assert!(answered.contains(&first) && answered.contains(&second));
    drop(connection);

    // A connection that does not greet first is ended.
    let mut ungreeted = UnixStream::connect(&paths.public).unwrap();
    let opening = signal::Dispatch::Open(signal::Opening { exchange: first, query: call(wire::Input::Resolve(Vec::new())) });
    ungreeted.write_frame(&FrameBody::from(opening.signalize().unwrap().bytes().to_vec()), capacity()).unwrap();
    ungreeted.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut ending = Vec::new();
    std::io::Read::read_to_end(&mut ungreeted, &mut ending).unwrap();
    assert!(!ending.is_empty(), "the connection is ended with the fault it names");

    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn a_subscription_streams_its_frames_and_the_state_survives_a_restart() {
    let directory = tempfile::tempdir().unwrap();
    let bootstrap = bootstrap(place(directory.path(), "stream"));
    let paths = SocketPaths::from(&bootstrap.place);
    serve_emissions(&paths.translator);
    let mut child = spawn_nexus(&bootstrap);
    wait_for(&paths.meta);
    wait_for(&paths.public);
    for (revision, set) in ready_sets(Vec::new()).into_iter().enumerate() {
        assert!(matches!(configure(&paths.meta, revision as i64, set), meta::Output::Configured(_)));
    }
    let repository = directory.path().join("beliefs");
    repository_fixture(&repository);
    let first = directory.path().join("first");
    let second = directory.path().join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    let registered = public(&paths.public, own_caller(), wire::Input::Register(wire::Repository(wire::RepositoryPath(repository.to_string_lossy().into_owned()))));
    assert!(matches!(registered, wire::Output::Registered(_)), "{registered:?}");
    let furnished = public(&paths.public, own_caller(), furnish_request(&first, wire::Harness::Codex, &[(wire::Kind::Spirit, "core")]));
    assert!(matches!(furnished, wire::Output::Furnished(_)), "{furnished:?}");

    let stream = UnixStream::connect(&paths.public).unwrap();
    let mut subscription = QueryingConnection::<public_call::Input, public_call::Output>::greeted(
        stream,
        Patience::Until(Instant::now() + Duration::from_secs(30)),
        capacity(),
    )
    .unwrap();
    let exchange = subscription
        .open(public_call::Input::Call(public_call::Call_Data { caller: own_caller(), query: wire::Input::Subscribe }))
        .unwrap();
    let mut frame = || match subscription.delivery().unwrap() {
        Some(Delivery::Answer(answer)) if answer.exchange == exchange => match answer.response {
            public_call::Output::Answered(wire::Output::Subscribed(frame)) => frame,
            other => panic!("a subscription answered something else: {other:?}"),
        },
        _ => panic!("the subscription gave no frame"),
    };
    let signal::Frame::Opened(wire::Workspaces(opened)) = frame() else { panic!("the first frame is not Opened") };
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].workspace.0 .0, first.to_string_lossy());

    let furnished = public(&paths.public, own_caller(), furnish_request(&second, wire::Harness::Codex, &[(wire::Kind::Spirit, "core")]));
    assert!(matches!(furnished, wire::Output::Furnished(_)), "{furnished:?}");
    let signal::Frame::Changed(wire::Furnishing::Furnished(changed)) = frame() else { panic!("the next frame is not the furnishing") };
    assert_eq!(changed.workspace.0 .0, second.to_string_lossy());
    drop(subscription);

    child.kill().unwrap();
    child.wait().unwrap();
    for socket in [&paths.public, &paths.meta] {
        if socket.exists() {
            std::fs::remove_file(socket).unwrap();
        }
    }
    let mut restarted = spawn_nexus(&bootstrap);
    wait_for(&paths.public);
    let resolved = public(&paths.public, own_caller(), wire::Input::Resolve(vec![wire::ModuleName { kind: wire::Kind::Spirit, subject: wire::Subject("core".into()) }]));
    let wire::Output::Resolved(resolution) = resolved else { panic!("the restarted Nexus did not resolve: {resolved:?}") };
    assert_eq!(resolution.found.0.len(), 1, "the registered repository survived the restart");
    restarted.kill().unwrap();
    restarted.wait().unwrap();
}
