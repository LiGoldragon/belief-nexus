//! Durable orchestration for Belief.
//!
//! Transports supply peer-bound callers and typed values. This crate owns
//! authorization, Memory ordering, source polling, publication recovery and
//! repeat-identifiable subscriptions.

use belief_nexus_core::{BeliefCore, PublicationContext, RecoveryOutcome, ResolveError};
use emission_belief_nexus as emission;
use meta_signal_belief_nexus as meta;
use rkyv::{Archive, Deserialize, Serialize};
use signal::{ByteViewable, Restorable, Signal, Signalizable};
use signal_belief_nexus as wire;
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

pub mod server;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonBootstrap {
    pub nexus: meta::NexusBootstrap,
}

fn rebuild_core(memory: &MemoryImage) -> Result<BeliefCore, ()> {
    let mut core = BeliefCore::new();
    for repository in &memory.repositories {
        core.observe(repository.snapshot.clone().into())
            .map_err(|_| ())?;
    }
    for workspace in &memory.workspaces {
        if let Some(successful) = &workspace.successful {
            if !core.restore_furnished(
                workspace.request.clone(),
                successful.clone(),
                workspace
                    .successful_basis
                    .iter()
                    .map(|state| (state.repository.clone(), state.source_revision.clone()))
                    .collect(),
            ) {
                return Err(());
            }
        }
        if let Some(pending) = &workspace.pending {
            let defaults = workspace.placement_defaults.iter().map(|value| (value.kind.clone(), value.placement.clone())).collect();
            let plan = core
                .prepare_furnish_with_defaults(workspace.request.clone(), &defaults)
                .map_err(|_| ())?;
            let context = PublicationContext {
                owner: workspace.owner.clone(),
                configuration_revision: workspace.configuration_revision.clone(),
                layout: workspace.layout.clone(),
                retention_policy: workspace.retention.clone(),
            };
            if &core.begin_publication(&plan, context).map_err(|_| ())? != pending {
                return Err(());
            }
        } else if workspace.successful.is_none() {
            return Err(());
        }
    }
    Ok(core)
}

fn validate_bootstrap(value: &meta::NexusBootstrap) -> Result<(), BootstrapError> {
    if value.ordinary_socket.0 .0 == value.meta_socket.0 .0
        || value.ordinary_socket.0 .0 == value.emission_translator_socket.0 .0
        || value.meta_socket.0 .0 == value.emission_translator_socket.0 .0
    {
        return Err(BootstrapError::SocketCollision);
    }
    if value.admission.byte_limit.0 <= 0 || value.admission.timeout.0 <= 0 {
        return Err(BootstrapError::InvalidAdmission);
    }
    Ok(())
}

fn peer_owner(peers: &wire::PeerList, caller: &translator::Caller) -> Option<wire::Owner> {
    peers.0.iter().find(|peer| peer.peer_identity.user_id == caller.user_id && peer.peer_identity.process_id == caller.process_id)
        .map(|peer| wire::Owner::Peer(peer.peer_id.clone()))
}

fn same_owner(first: &wire::Owner, second: &wire::Owner) -> bool {
    first == second
}

fn setting_name(set: &meta::Set) -> meta::SettingName {
    match set {
        meta::Set::RepositoryPolling(_) => meta::SettingName::RepositoryPolling,
        meta::Set::SubscriberCapacity(_) => meta::SettingName::SubscriberCapacity,
        meta::Set::OrdinaryPeers(_) => meta::SettingName::OrdinaryPeers,
        meta::Set::MetaPeers(_) => meta::SettingName::MetaPeers,
        meta::Set::Layout(value) => meta::SettingName::Layout(value.harness.clone()),
        meta::Set::Placement(value) => meta::SettingName::Placement(value.kind.clone()),
        meta::Set::GenerationRetention(_) => meta::SettingName::GenerationRetention,
        meta::Set::NexusFrameCapacity(_) => meta::SettingName::NexusFrameCapacity,
        meta::Set::NexusTransportTimeout(_) => meta::SettingName::NexusTransportTimeout,
        meta::Set::NexusSocket(_) => meta::SettingName::NexusSocket,
        meta::Set::MetaNexusSocket(_) => meta::SettingName::MetaNexusSocket,
        meta::Set::CallTimeout(_) => meta::SettingName::CallTimeout,
        meta::Set::OutputLimit(_) => meta::SettingName::OutputLimit,
        meta::Set::TextBudget(_) => meta::SettingName::TextBudget,
        meta::Set::TranslatorFrameCapacity(_) => meta::SettingName::TranslatorFrameCapacity,
        meta::Set::TranslatorTransportTimeout(_) => meta::SettingName::TranslatorTransportTimeout,
        meta::Set::AllowedNexusPeers(_) => meta::SettingName::AllowedNexusPeers,
        meta::Set::GitExecutable(_) => meta::SettingName::GitExecutable,
    }
}

fn setting_owner(set: &meta::Set) -> meta::Keeper {
    match set {
        meta::Set::NexusSocket(_) | meta::Set::MetaNexusSocket(_) | meta::Set::CallTimeout(_)
        | meta::Set::OutputLimit(_) | meta::Set::TextBudget(_) | meta::Set::TranslatorFrameCapacity(_)
        | meta::Set::TranslatorTransportTimeout(_) | meta::Set::AllowedNexusPeers(_)
        | meta::Set::GitExecutable(_) => meta::Keeper::Translator,
        _ => meta::Keeper::Nexus,
    }
}

fn emission_mismatch(query: &emission::Query) -> emission::Response {
    let diagnostic = wire::Diagnostic { executable_path: wire::ExecutablePath(String::new()), exit_status_option: None, string: "emission outcome configuration revision mismatch".into() };
    match query {
        emission::Query::Snapshot(repository) => emission::Response::EmissionRefused(emission::EmissionRefusal::Source(emission::SourceRefusal { repository: repository.clone(), source_operation: emission::SourceOperation::ReadingMain, source_evidence: emission::SourceEvidence::Translator(diagnostic) })),
        emission::Query::Publish(value) => emission::Response::EmissionRefused(emission::EmissionRefusal::Publication(emission::PublicationRefusal { workspace: value.workspace.clone(), publication_operation: emission::PublicationOperation::StagingGeneration, publication_evidence: emission::PublicationEvidence::Translator(diagnostic) })),
        emission::Query::InspectPublication(value) => emission::Response::EmissionRefused(emission::EmissionRefusal::Publication(emission::PublicationRefusal { workspace: value.workspace.clone(), publication_operation: emission::PublicationOperation::InspectingPointer, publication_evidence: emission::PublicationEvidence::Translator(diagnostic) })),
        emission::Query::Clean(value) => emission::Response::CleanupRefused(wire::FileFailure { workspace: value.workspace.clone(), file_operation: wire::FileOperation::Cleaning, file_evidence: wire::FileEvidence::Translator(diagnostic) }),
    }
}

fn resolve_refusal(error: ResolveError) -> wire::Refusal {
    match error {
        ResolveError::Ambiguous { name, repositories } => {
            let repository = repositories
                .first()
                .cloned()
                .unwrap_or_else(|| wire::Repository(wire::RepositoryPath(String::new())));
            wire::Refusal::Source(wire::SourceFailure {
                repository,
                source_operation: wire::SourceOperation::Resolving,
                source_evidence: wire::SourceEvidence::Ambiguous(wire::Ambiguous_Data {
                    module_name: name,
                    repository_vector: repositories,
                }),
            })
        }
        ResolveError::MissingPlacement(_) => wire::Refusal::NotReady,
    }
}

fn map_source(
    refusal: emission::SourceRefusal,
    operation: wire::SourceOperation,
) -> wire::SourceFailure {
    wire::SourceFailure {
        repository: refusal.repository,
        source_operation: operation,
        source_evidence: match refusal.source_evidence {
            emission::SourceEvidence::NotRepository => wire::SourceEvidence::NotRepository,
            emission::SourceEvidence::MainUnavailable => wire::SourceEvidence::MainUnavailable,
            emission::SourceEvidence::RevisionUnavailable => {
                wire::SourceEvidence::RevisionUnavailable
            }
            emission::SourceEvidence::InvalidModule(value) => wire::SourceEvidence::InvalidModule(wire::InvalidModule {
                module_name: value.module_name,
                relative_path: value.relative_path,
                invalid_placement: match value.invalid_placement {
                    emission::InvalidPlacement::Unknown(value) => wire::InvalidPlacement::Unknown(value),
                    emission::InvalidPlacement::NonScalar => wire::InvalidPlacement::NonScalar,
                    emission::InvalidPlacement::Duplicate => wire::InvalidPlacement::Duplicate,
                },
            }),
            emission::SourceEvidence::Translator(value) => wire::SourceEvidence::Translator(value),
        },
    }
}

fn map_emission(refusal: emission::EmissionRefusal) -> wire::Refusal {
    match refusal {
        emission::EmissionRefusal::Source(value) => {
            wire::Refusal::Source(map_source(value, wire::SourceOperation::Snapshotting))
        }
        emission::EmissionRefusal::Publication(value) => wire::Refusal::File(wire::FileFailure {
            workspace: value.workspace,
            file_operation: wire::FileOperation::Staging,
            file_evidence: wire::FileEvidence::PermissionDenied,
        }),
    }
}

fn file_refusal(workspace: wire::Workspace) -> wire::Refusal {
    wire::Refusal::File(wire::FileFailure {
        workspace,
        file_operation: wire::FileOperation::Staging,
        file_evidence: wire::FileEvidence::PermissionDenied,
    })
}

fn memory_failure(operation: wire::MemoryOperation, error: MemoryError) -> wire::MemoryFailure {
    wire::MemoryFailure {
        memory_operation: operation,
        memory_evidence: match error {
            MemoryError::Unavailable => wire::MemoryEvidence::Unavailable,
            MemoryError::Corrupt => wire::MemoryEvidence::Corrupt,
        },
    }
}

fn memory_refusal(operation: wire::MemoryOperation) -> wire::Refusal {
    wire::Refusal::Memory(memory_failure(operation, MemoryError::Unavailable))
}

fn refusal_kind(refusal: &wire::Refusal) -> wire::RefusalKind {
    match refusal {
        wire::Refusal::Source(_) => wire::RefusalKind::Source,
        wire::Refusal::File(_) => wire::RefusalKind::File,
        wire::Refusal::Memory(_) => wire::RefusalKind::Memory,
        wire::Refusal::NotAuthorized(_) => wire::RefusalKind::NotAuthorized,
        wire::Refusal::SubscriberCapacityReached(_) => {
            wire::RefusalKind::SubscriberCapacityReached
        }
        wire::Refusal::NotReady => wire::RefusalKind::NotReady,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WatchFailure {
    repository: wire::Repository,
    source_revision_option: Option<wire::SourceRevision>,
    source_failure: wire::SourceFailure,
}

fn watch_unavailable(
    repository: wire::Repository,
    revision: Option<wire::SourceRevision>,
) -> WatchFailure {
    WatchFailure {
        repository: repository.clone(),
        source_revision_option: revision,
        source_failure: wire::SourceFailure {
            repository,
            source_operation: wire::SourceOperation::Watching,
            source_evidence: wire::SourceEvidence::MainUnavailable,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeEffects {
        replies: VecDeque<emission::Response>,
        calls: Vec<emission::Query>,
        publish_from_query: bool,
        cleanup_failures: usize,
    }

    impl EmissionPort for FakeEffects {
        fn emit(&mut self, attempt: emission::EmissionAttempt) -> emission::EmissionAttemptOutcome {
            let revision = attempt.configuration_revision;
            let query = attempt.query;
            self.calls.push(query.clone());
            let response = if let Some(reply) = self.replies.pop_front() {
                reply
            } else {
                match query {
                    emission::Query::Publish(publication) if self.publish_from_query => emission::Response::Published(emission::PublicationState {
                        workspace: publication.workspace,
                        publication_identity: emission::PublicationIdentity {
                            generation_id: publication.generation_id,
                            publication_revision: publication.intended_publication.0,
                        },
                    }),
                    emission::Query::Clean(request) if self.cleanup_failures > 0 => {
                        self.cleanup_failures -= 1;
                        emission::Response::CleanupRefused(wire::FileFailure { workspace: request.workspace, file_operation: wire::FileOperation::Cleaning, file_evidence: wire::FileEvidence::PermissionDenied })
                    }
                    emission::Query::Clean(request) => emission::Response::Cleaned(emission::CleanupState { workspace: request.workspace, publication_identity: request.publication_identity, cleanup_outcome: emission::CleanupOutcome::Removed }),
                    _ => panic!("missing effect reply"),
                }
            };
            emission::EmissionAttemptOutcome { configuration_revision: revision, response }
        }
    }

    #[derive(Default)]
    struct FakeMemory {
        image: Option<MemoryImage>,
        stores: usize,
        fail_at: Option<usize>,
        commit_then_panic_at: Option<usize>,
        committed: Option<Arc<Mutex<Option<MemoryImage>>>>,
    }

    impl MemoryStore for FakeMemory {
        fn load(&mut self) -> Result<Option<MemoryImage>, MemoryError> {
            Ok(self.image.clone())
        }

        fn store(&mut self, image: &MemoryImage) -> Result<(), MemoryError> {
            self.stores += 1;
            if self.fail_at == Some(self.stores) {
                return Err(MemoryError::Unavailable);
            }
            let image = image.clone();
            self.image = Some(image.clone());
            if let Some(committed) = &self.committed {
                *committed.lock().unwrap() = Some(image);
            }
            if self.commit_then_panic_at == Some(self.stores) {
                panic!("simulated process loss after durable commit");
            }
            Ok(())
        }
    }

    fn caller() -> translator::Caller {
        translator::Caller {
            user_id: translator::UserId(1000),
            process_id: translator::ProcessId(44),
        }
    }

    fn peer() -> translator::PeerBinding {
        translator::PeerBinding {
            peer_id: translator::PeerId("client".into()),
            peer_identity: translator::PeerIdentity {
                user_id: translator::UserId(1000),
                process_id: translator::ProcessId(44),
            },
        }
    }

    fn bootstrap() -> DaemonBootstrap {
        DaemonBootstrap {
            nexus: meta::NexusBootstrap {
                ordinary_socket: meta::OrdinarySocket(meta::SocketPath("/run/ordinary".into())),
                meta_socket: meta::MetaSocket(meta::SocketPath("/run/meta".into())),
                emission_translator_socket: meta::EmissionTranslatorSocket(meta::SocketPath(
                    "/run/translator".into(),
                )),
                memory_path: meta::MemoryPath("/unused".into()),
                admission: meta::Admission {
                    byte_limit: meta::ByteLimit(1_000_000),
                    timeout: meta::Timeout(5_000),
                },
                paired_translator: meta::PairedTranslator {
                    user_id: translator::UserId(1000),
                    group_id: translator::GroupId(1000),
                    process_id: translator::ProcessId(44),
                },
                bootstrap_owner: meta::BootstrapOwner(wire::Owner::Login(translator::UserId(1000))),
            },
        }
    }

    fn layout() -> meta::Layout {
        meta::Layout {
            owned_root: meta::OwnedRoot(wire::RelativePath(".belief".into())),
            generations_directory: meta::GenerationsDirectory(wire::RelativePath(
                "generations".into(),
            )),
            current_link: meta::CurrentLink(wire::RelativePath("current".into())),
            settings_file: meta::SettingsFile(wire::RelativePath(".codex/config.toml".into())),
            instructions_setting: meta::InstructionsSetting("model_instructions_file".into()),
            instructions_file: meta::InstructionsFile(wire::RelativePath("instructions.md".into())),
            skills_directory: meta::SkillsDirectory(wire::RelativePath(".agents/skills".into())),
        }
    }

    fn ready_memory() -> MemoryImage {
        let peer = peer();
        MemoryImage {
            owner: wire::Owner::Login(translator::UserId(1000)),
            configuration: Configuration {
                revision: meta::ConfigurationRevision(4),
                repository_polling: Some(meta::Timeout(100)),
                subscriber_capacity: Some(1),
                ordinary_peers: Some(wire::PeerList(vec![peer.clone()])),
                meta_peers: Some(wire::PeerList(vec![peer.clone()])),
                layouts: vec![meta::HarnessLayout { harness: wire::Harness::Codex, layout: layout() }],
                placements: vec![
                    meta::PlacementDefault { kind: wire::Kind::Spirit, placement: wire::Placement::SystemPrompt },
                    meta::PlacementDefault { kind: wire::Kind::Intent, placement: wire::Placement::SystemPrompt },
                    meta::PlacementDefault { kind: wire::Kind::Vision, placement: wire::Placement::Skills },
                    meta::PlacementDefault { kind: wire::Kind::Knowledge, placement: wire::Placement::Skills },
                    meta::PlacementDefault { kind: wire::Kind::Role, placement: wire::Placement::Skills },
                ],
                retention: meta::RetentionPolicy::KeepAll,
                nexus_frame_capacity: Some(meta::ByteLimit(1_000_000)),
                nexus_transport_timeout: Some(meta::Timeout(100)),
                translator_sets: vec![
                    meta::Set::NexusSocket(meta::SocketPath("ordinary".into())),
                    meta::Set::MetaNexusSocket(meta::SocketPath("meta".into())),
                    meta::Set::CallTimeout(meta::Timeout(100)),
                    meta::Set::OutputLimit(meta::ByteLimit(1_000_000)),
                    meta::Set::TextBudget(meta::Budget { node_limit: meta::NodeLimit(1000), depth_limit: meta::DepthLimit(100) }),
                    meta::Set::TranslatorFrameCapacity(meta::ByteLimit(1_000_000)),
                    meta::Set::TranslatorTransportTimeout(meta::Timeout(100)),
                    meta::Set::AllowedNexusPeers(wire::PeerList(vec![peer.clone()])),
                    meta::Set::GitExecutable(wire::ExecutablePath("git".into())),
                ],
            },
            repositories: Vec::new(),
            workspaces: Vec::new(),
            cleanup_maintenance: Vec::new(),
            next_event_sequence: 1,
        }
    }

    fn module() -> wire::ModuleName {
        wire::ModuleName {
            kind: wire::Kind::Vision,
            subject: wire::Subject("nexus".into()),
        }
    }

    fn repository() -> wire::Repository {
        wire::Repository(wire::RepositoryPath("/repo".into()))
    }

    fn other_repository() -> wire::Repository {
        wire::Repository(wire::RepositoryPath("/z-repo".into()))
    }

    fn workspace() -> wire::Workspace {
        wire::Workspace(wire::WorkspacePath("/work".into()))
    }

    fn workspace_named(name: &str) -> wire::Workspace {
        wire::Workspace(wire::WorkspacePath(format!("/work/{name}")))
    }

    fn snapshot(revision: &str, content: &str) -> emission::SourceSnapshot {
        emission::SourceSnapshot {
            repository: repository(),
            source_revision: wire::SourceRevision(revision.into()),
            module_source_vector: vec![emission::ModuleSource {
                module_name: module(),
                relative_path: wire::RelativePath("vision/nexus.md".into()),
                placement_option: None,
                content: emission::Content(content.into()),
            }],
        }
    }

    fn other_snapshot(revision: &str, content: &str) -> emission::SourceSnapshot {
        emission::SourceSnapshot {
            repository: other_repository(),
            source_revision: wire::SourceRevision(revision.into()),
            module_source_vector: vec![emission::ModuleSource {
                module_name: wire::ModuleName {
                    kind: wire::Kind::Knowledge,
                    subject: wire::Subject("unrelated".into()),
                },
                relative_path: wire::RelativePath("knowledge/unrelated.md".into()),
                placement_option: None,
                content: emission::Content(content.into()),
            }],
        }
    }

    fn conflicting_snapshot(revision: &str, content: &str) -> emission::SourceSnapshot {
        emission::SourceSnapshot {
            repository: other_repository(),
            source_revision: wire::SourceRevision(revision.into()),
            module_source_vector: vec![emission::ModuleSource {
                module_name: module(),
                relative_path: wire::RelativePath("vision/nexus.md".into()),
                placement_option: None,
                content: emission::Content(content.into()),
            }],
        }
    }

    fn furnish_request() -> wire::FurnishRequest {
        wire::FurnishRequest {
            workspace: workspace(),
            harness: wire::Harness::Codex,
            module_name_vector: vec![module()],
        }
    }

    fn furnish_request_named(name: &str) -> wire::FurnishRequest {
        wire::FurnishRequest {
            workspace: workspace_named(name),
            harness: wire::Harness::Codex,
            module_name_vector: vec![module()],
        }
    }

    fn peer_owner(name: &str) -> wire::Owner {
        wire::Owner::Peer(translator::PeerId(name.into()))
    }

    fn register(daemon: &mut Daemon<FakeEffects, FakeMemory>) {
        let response = daemon.handle(wire::Call {
            caller: caller(),
            query: wire::Query::Register(repository()),
        });
        assert!(matches!(response, wire::Response::Registered(_)));
    }

    #[test]
    fn login_owner_survives_cli_pid_changes_and_meta_rejects_other_login() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ].into(),
            publish_from_query: true,
            ..Default::default()
        };
        let memory = FakeMemory { image: Some(ready_memory()), ..Default::default() };
        let mut daemon = Daemon::start(bootstrap(), effects, memory).unwrap();
        let registered = daemon.handle(wire::Call { caller: caller(), query: wire::Query::Register(repository()) });
        assert!(matches!(registered, wire::Response::Registered(_)));
        let second = translator::Caller { user_id: translator::UserId(1000), process_id: translator::ProcessId(999) };
        let furnished = daemon.handle(wire::Call { caller: second.clone(), query: wire::Query::Furnish(furnish_request()) });
        assert!(matches!(furnished, wire::Response::Furnished(_)), "same login lost ownership: {furnished:?}");

        let current = daemon.memory.configuration.revision.clone();
        let configured = daemon.configure(meta::MetaCall { caller: second, query: meta::Query::Configure(meta::ConfigureRequest {
            configuration_revision_option: Some(current), set: meta::Set::RepositoryPolling(meta::Timeout(200)),
        }) });
        assert!(matches!(configured, meta::Response::Configured(_)));
        let unconditional = daemon.configure(meta::MetaCall { caller: translator::Caller { user_id: translator::UserId(1000), process_id: translator::ProcessId(1001) }, query: meta::Query::Configure(meta::ConfigureRequest {
            configuration_revision_option: None, set: meta::Set::RepositoryPolling(meta::Timeout(250)),
        }) });
        let expected_revision = daemon.memory.configuration.revision.clone();
        assert!(matches!(unconditional, meta::Response::Configured(meta::ConfiguredState { configuration_revision, .. }) if configuration_revision == expected_revision));
        let refused = daemon.configure(meta::MetaCall { caller: translator::Caller { user_id: translator::UserId(2000), process_id: translator::ProcessId(999) }, query: meta::Query::Configure(meta::ConfigureRequest {
            configuration_revision_option: Some(daemon.memory.configuration.revision.clone()), set: meta::Set::RepositoryPolling(meta::Timeout(300)),
        }) });
        assert!(matches!(refused, meta::Response::ConfigureRefused(meta::ConfigurationRefusal::NotAuthorized(_))));
    }

    #[test]
    fn exhausted_configuration_revision_refuses_before_mutating_persisted_memory() {
        let mut image = ready_memory();
        image.configuration.revision = meta::ConfigurationRevision(i64::MAX);
        let expected = image.clone();
        let memory = FakeMemory { image: Some(image), ..Default::default() };
        let mut daemon = Daemon::start(bootstrap(), FakeEffects::default(), memory).unwrap();

        let response = daemon.configure(meta::MetaCall {
            caller: caller(),
            query: meta::Query::Configure(meta::ConfigureRequest {
                configuration_revision_option: Some(meta::ConfigurationRevision(i64::MAX)),
                set: meta::Set::RepositoryPolling(meta::Timeout(200)),
            }),
        });

        assert!(matches!(response, meta::Response::ConfigureRefused(meta::ConfigurationRefusal::Unavailable)));
        assert_eq!(daemon.memory, expected);
        assert_eq!(daemon.store.image, Some(expected));
        assert_eq!(daemon.store.stores, 0);
    }

    #[test]
    fn configured_direct_peers_are_isolated_by_peer_id() {
        let mut image = ready_memory();
        let peer_two = translator::PeerBinding { peer_id: translator::PeerId("other".into()), peer_identity: translator::PeerIdentity { user_id: translator::UserId(1000), process_id: translator::ProcessId(55) } };
        image.configuration.ordinary_peers.as_mut().unwrap().0.push(peer_two);
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ].into(), ..Default::default()
        };
        let memory = FakeMemory { image: Some(image), ..Default::default() };
        let mut daemon = Daemon::start(bootstrap(), effects, memory).unwrap();
        let first = wire::Owner::Peer(translator::PeerId("client".into()));
        let registered = daemon.handle_as(wire::Call { caller: caller(), query: wire::Query::Register(repository()) }, first);
        assert!(matches!(registered, wire::Response::Registered(_)));
        let second_caller = translator::Caller { user_id: translator::UserId(1000), process_id: translator::ProcessId(55) };
        let response = daemon.handle_as(wire::Call { caller: second_caller, query: wire::Query::Resolve(vec![module()]) }, wire::Owner::Peer(translator::PeerId("other".into())));
        assert!(matches!(response, wire::Response::Refused(wire::Refusal::NotAuthorized(_))));
    }

    #[test]
    fn file_memory_round_trips_typed_state() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("memory.signal");
        let mut memory = FileMemory::new(path.clone());
        let image = ready_memory();
        memory.store(&image).unwrap();
        assert_eq!(memory.load().unwrap(), Some(image));
        assert!(path.exists());
    }

    #[test]
    fn register_observes_main_again_before_returning() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("two", "two")),
            ]
            .into(),
            ..Default::default()
        };
        let memory = FakeMemory {
            image: Some(ready_memory()),
            ..Default::default()
        };
        let mut daemon = Daemon::start(bootstrap(), effects, memory).unwrap();
        let response = daemon.handle(wire::Call {
            caller: caller(),
            query: wire::Query::Register(repository()),
        });
        assert!(
            matches!(response, wire::Response::Registered(wire::RepositoryState { source_revision: wire::SourceRevision(ref value), .. }) if value == "two")
        );
        assert_eq!(
            daemon.memory.repositories[0].snapshot.source_revision,
            wire::SourceRevision("two".into())
        );
    }

    #[test]
    fn register_rejects_a_snapshot_for_an_unrequested_repository_before_mutation() {
        let effects = FakeEffects {
            replies: [emission::Response::Snapshotted(other_snapshot("one", "other"))].into(),
            ..Default::default()
        };
        let initial = ready_memory();
        let memory = FakeMemory {
            image: Some(initial.clone()),
            ..Default::default()
        };
        let mut daemon = Daemon::start(bootstrap(), effects, memory).unwrap();

        let response = daemon.handle(wire::Call {
            caller: caller(),
            query: wire::Query::Register(repository()),
        });

        assert!(matches!(response, wire::Response::Refused(wire::Refusal::NotReady)));
        assert_eq!(daemon.memory, initial);
        assert_eq!(daemon.store.stores, 0);
        assert_eq!(daemon.effects.calls, vec![emission::Query::Snapshot(repository())]);
    }

    #[test]
    fn register_handoff_ignores_a_snapshot_for_another_repository() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(other_snapshot("two", "other")),
            ]
            .into(),
            ..Default::default()
        };
        let memory = FakeMemory {
            image: Some(ready_memory()),
            ..Default::default()
        };
        let mut daemon = Daemon::start(bootstrap(), effects, memory).unwrap();

        let response = daemon.handle(wire::Call {
            caller: caller(),
            query: wire::Query::Register(repository()),
        });

        assert!(matches!(
            response,
            wire::Response::Registered(wire::RepositoryState {
                repository: ref value,
                source_revision: wire::SourceRevision(ref revision),
            }) if value == &repository() && revision == "one"
        ));
        assert_eq!(daemon.memory.repositories.len(), 1);
        assert_eq!(daemon.memory.repositories[0].snapshot, snapshot("one", "one"));
        assert_eq!(daemon.store.stores, 1);
        assert_eq!(daemon.store.image, Some(daemon.memory.clone()));
    }

    #[test]
    fn pending_is_durable_before_publish_and_post_pointer_failure_keeps_it() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let memory = FakeMemory {
            image: Some(ready_memory()),
            ..Default::default()
        };
        let mut daemon = Daemon::start(bootstrap(), effects, memory).unwrap();
        register(&mut daemon);
        daemon.store.fail_at = Some(daemon.store.stores + 2);
        let response = daemon.handle(wire::Call {
            caller: caller(),
            query: wire::Query::Furnish(furnish_request()),
        });
        assert!(matches!(
            response,
            wire::Response::MemoryAfterPublication(_)
        ));
        assert!(daemon.memory.workspaces[0].pending.is_some());
        assert!(daemon.store.image.as_ref().unwrap().workspaces[0]
            .pending
            .is_some());
    }

    #[test]
    fn exact_pending_recovers_after_restart() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let memory = FakeMemory {
            image: Some(ready_memory()),
            ..Default::default()
        };
        let mut first = Daemon::start(bootstrap(), effects, memory).unwrap();
        register(&mut first);
        first.store.fail_at = Some(first.store.stores + 2);
        first.handle(wire::Call {
            caller: caller(),
            query: wire::Query::Furnish(furnish_request()),
        });
        let durable = first.store.image.clone().unwrap();
        let pending = durable.workspaces[0].pending.clone().unwrap();
        let inspection = emission::PublicationInspection {
            workspace: workspace(),
            publication_identity_option: Some(emission::PublicationIdentity {
                generation_id: pending.generation_id.clone(),
                publication_revision: pending.intended_publication.0.clone(),
            }),
            publication_manifest_option: Some(emission::PublicationManifest {
                generation_id: pending.generation_id,
                intended_publication: pending.intended_publication,
                repository_states: pending.repository_states,
                found_modules: pending.found_modules,
                missing_modules: pending.missing_modules,
            }),
        };
        let effects = FakeEffects {
            replies: [emission::Response::Inspected(inspection)].into(),
            ..Default::default()
        };
        let memory = FakeMemory {
            image: Some(durable),
            ..Default::default()
        };
        let second = Daemon::start(bootstrap(), effects, memory).unwrap();
        assert!(second.memory.workspaces[0].pending.is_none());
        assert!(second.memory.workspaces[0].successful.is_some());
        assert!(second.effects.calls.iter().all(|query| {
            !matches!(query, emission::Query::Publish(_))
        }));
    }

    #[test]
    fn startup_reconciles_the_source_commit_crash_window() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let memory = FakeMemory { image: Some(ready_memory()), ..Default::default() };
        let mut daemon = Daemon::start(bootstrap(), effects, memory).unwrap();
        register(&mut daemon);
        assert!(matches!(
            daemon.handle(wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request()) }),
            wire::Response::Furnished(_)
        ));
        let old = daemon.memory.workspaces[0].successful.clone().unwrap();
        let committed = Arc::new(Mutex::new(None));
        daemon.store.committed = Some(committed.clone());
        daemon.store.commit_then_panic_at = Some(daemon.store.stores + 1);
        daemon.effects.replies.push_back(emission::Response::Snapshotted(snapshot("two", "two")));
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| daemon.poll_once()));
        assert!(panic.is_err(), "fixture must stop after the newer source snapshot is durable");

        let durable = committed.lock().unwrap().clone().unwrap();
        assert_eq!(durable.repositories[0].snapshot.source_revision, wire::SourceRevision("two".into()));
        assert_eq!(durable.workspaces[0].successful, Some(old.clone()));
        assert!(durable.workspaces[0].pending.is_none());

        let effects = FakeEffects { publish_from_query: true, ..Default::default() };
        let restarted = Daemon::start(
            bootstrap(),
            effects,
            FakeMemory { image: Some(durable), ..Default::default() },
        )
        .unwrap();
        let recovered = restarted.memory.workspaces[0].successful.as_ref().unwrap();
        assert_ne!(recovered.generation_id, old.generation_id);
        assert_eq!(recovered.publication_revision, wire::PublicationRevision(2));
        assert!(restarted.memory.workspaces[0].pending.is_none());
        assert_eq!(
            restarted.effects.calls.iter().filter(|query| matches!(query, emission::Query::Publish(_))).count(),
            1
        );
    }

    #[test]
    fn startup_reconciles_each_affected_workspace_once() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let mut daemon = Daemon::start(
            bootstrap(),
            effects,
            FakeMemory { image: Some(ready_memory()), ..Default::default() },
        )
        .unwrap();
        register(&mut daemon);
        for name in ["first", "second"] {
            assert!(matches!(
                daemon.handle(wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request_named(name)) }),
                wire::Response::Furnished(_)
            ));
        }
        let mut durable = daemon.memory.clone();
        durable.repositories[0].snapshot = snapshot("two", "two");

        let restarted = Daemon::start(
            bootstrap(),
            FakeEffects { publish_from_query: true, ..Default::default() },
            FakeMemory { image: Some(durable), ..Default::default() },
        )
        .unwrap();
        assert_eq!(
            restarted.effects.calls.iter().filter(|query| matches!(query, emission::Query::Publish(_))).count(),
            2
        );
        assert!(restarted.memory.workspaces.iter().all(|workspace| {
            workspace.pending.is_none()
                && workspace.successful.as_ref().is_some_and(|value| value.publication_revision == wire::PublicationRevision(2))
        }));
    }

    #[test]
    fn ambiguous_settled_workspace_starts_and_preserves_its_success() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let mut daemon = Daemon::start(
            bootstrap(),
            effects,
            FakeMemory { image: Some(ready_memory()), ..Default::default() },
        )
        .unwrap();
        register(&mut daemon);
        assert!(matches!(
            daemon.handle(wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request()) }),
            wire::Response::Furnished(_)
        ));
        let prior = daemon.memory.workspaces[0].successful.clone().unwrap();
        let mut durable = daemon.memory.clone();
        durable.repositories.push(StoredRepository {
            owner: wire::Owner::Login(caller().user_id),
            snapshot: conflicting_snapshot("other", "ambiguous"),
        });

        let restarted = Daemon::start(
            bootstrap(),
            FakeEffects::default(),
            FakeMemory { image: Some(durable), ..Default::default() },
        )
        .expect("settled success with newly ambiguous sources is retry refusal, not corrupt Memory");
        assert_eq!(restarted.memory.workspaces[0].successful, Some(prior));
        assert!(restarted.memory.workspaces[0].pending.is_none());
        assert!(restarted.events.iter().any(|event| matches!(
            event.response,
            wire::Response::RefurnishFailed(wire::RefurnishFailure { refusal_kind: wire::RefusalKind::Source, .. })
        )));
        assert!(restarted.effects.calls.iter().all(|query| !matches!(query, emission::Query::Publish(_))));
    }

    #[test]
    fn startup_retry_memory_failure_is_typed_and_the_next_restart_retries() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let mut daemon = Daemon::start(
            bootstrap(),
            effects,
            FakeMemory { image: Some(ready_memory()), ..Default::default() },
        )
        .unwrap();
        register(&mut daemon);
        assert!(matches!(
            daemon.handle(wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request()) }),
            wire::Response::Furnished(_)
        ));
        let prior = daemon.memory.workspaces[0].successful.clone().unwrap();
        let mut durable = daemon.memory.clone();
        durable.repositories[0].snapshot = snapshot("two", "two");

        let failed = Daemon::start(
            bootstrap(),
            FakeEffects { publish_from_query: true, ..Default::default() },
            FakeMemory { image: Some(durable), fail_at: Some(1), ..Default::default() },
        )
        .unwrap();
        assert_eq!(failed.memory.workspaces[0].successful, Some(prior));
        assert!(failed.memory.workspaces[0].pending.is_none());
        assert!(failed.events.iter().any(|event| matches!(
            event.response,
            wire::Response::RefurnishFailed(wire::RefurnishFailure { refusal_kind: wire::RefusalKind::Memory, .. })
        )));
        assert!(failed.effects.calls.iter().all(|query| !matches!(query, emission::Query::Publish(_))));

        let durable = failed.store.image.clone().unwrap();
        let recovered = Daemon::start(
            bootstrap(),
            FakeEffects { publish_from_query: true, ..Default::default() },
            FakeMemory { image: Some(durable), ..Default::default() },
        )
        .unwrap();
        assert_eq!(
            recovered.effects.calls.iter().filter(|query| matches!(query, emission::Query::Publish(_))).count(),
            1
        );
        assert!(recovered.memory.workspaces[0].pending.is_none());
        assert_eq!(
            recovered.memory.workspaces[0].successful.as_ref().unwrap().publication_revision,
            wire::PublicationRevision(2)
        );
    }

    #[test]
    fn current_successful_basis_is_quiet_on_startup() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let mut daemon = Daemon::start(
            bootstrap(),
            effects,
            FakeMemory { image: Some(ready_memory()), ..Default::default() },
        )
        .unwrap();
        register(&mut daemon);
        assert!(matches!(
            daemon.handle(wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request()) }),
            wire::Response::Furnished(_)
        ));

        let restarted = Daemon::start(
            bootstrap(),
            FakeEffects::default(),
            FakeMemory { image: Some(daemon.memory), ..Default::default() },
        )
        .unwrap();
        assert!(restarted.effects.calls.is_empty());
        assert!(restarted.memory.workspaces[0].pending.is_none());
    }

    #[test]
    fn unchanged_watch_is_quiet_and_subscriber_capacity_counts_active() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            ..Default::default()
        };
        let memory = FakeMemory {
            image: Some(ready_memory()),
            ..Default::default()
        };
        let mut daemon = Daemon::start(bootstrap(), effects, memory).unwrap();
        register(&mut daemon);
        assert!(daemon.poll_once().is_empty());
        let first = daemon.subscribe(wire::Owner::Login(caller().user_id)).unwrap();
        assert!(matches!(
            daemon.subscribe(wire::Owner::Login(caller().user_id)),
            Err(wire::Refusal::SubscriberCapacityReached(1))
        ));
        daemon.unsubscribe(first.id);
        assert!(daemon.subscribe(wire::Owner::Login(caller().user_id)).is_ok());
    }

    #[test]
    fn changed_watch_announces_only_successful_publication() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let memory = FakeMemory {
            image: Some(ready_memory()),
            ..Default::default()
        };
        let mut daemon = Daemon::start(bootstrap(), effects, memory).unwrap();
        register(&mut daemon);
        assert!(matches!(
            daemon.handle(wire::Call {
                caller: caller(),
                query: wire::Query::Furnish(furnish_request()),
            }),
            wire::Response::Furnished(_)
        ));
        daemon
            .effects
            .replies
            .push_back(emission::Response::Snapshotted(snapshot("two", "two")));
        let events = daemon.poll_once();
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0].response, wire::Response::Changed(_)));
        assert_eq!(
            daemon.memory.repositories[0].snapshot.source_revision,
            wire::SourceRevision("two".into())
        );
    }

    #[test]
    fn watch_mismatch_preserves_every_repository_and_workspace() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(other_snapshot("other-one", "other")),
                emission::Response::Snapshotted(other_snapshot("other-one", "other")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let memory = FakeMemory {
            image: Some(ready_memory()),
            ..Default::default()
        };
        let mut daemon = Daemon::start(bootstrap(), effects, memory).unwrap();
        register(&mut daemon);
        assert!(matches!(
            daemon.handle(wire::Call {
                caller: caller(),
                query: wire::Query::Register(other_repository()),
            }),
            wire::Response::Registered(_)
        ));
        assert!(matches!(
            daemon.handle(wire::Call {
                caller: caller(),
                query: wire::Query::Furnish(furnish_request()),
            }),
            wire::Response::Furnished(_)
        ));
        let before = daemon.memory.clone();
        let stores = daemon.store.stores;

        daemon.effects.replies.extend([
            emission::Response::Snapshotted(other_snapshot("other-two", "malformed")),
            emission::Response::Snapshotted(other_snapshot("other-one", "other")),
        ]);
        assert!(daemon.poll_once().is_empty());
        assert_eq!(daemon.memory, before);
        assert_eq!(daemon.store.stores, stores);
        assert_eq!(
            daemon.watch_failures,
            vec![watch_unavailable(repository(), Some(wire::SourceRevision("one".into())))]
        );
    }

    #[test]
    fn subscriptions_project_initial_and_future_events_to_workspace_owner() {
        let mut image = ready_memory();
        image.configuration.subscriber_capacity = Some(3);
        image.configuration.ordinary_peers.as_mut().unwrap().0.push(translator::PeerBinding {
            peer_id: translator::PeerId("other".into()),
            peer_identity: translator::PeerIdentity {
                user_id: translator::UserId(1000),
                process_id: translator::ProcessId(55),
            },
        });
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let mut daemon = Daemon::start(
            bootstrap(),
            effects,
            FakeMemory { image: Some(image), ..Default::default() },
        )
        .unwrap();
        register(&mut daemon);
        let first_owner = peer_owner("client");
        let second_owner = peer_owner("other");
        let login_owner = wire::Owner::Login(caller().user_id.clone());
        for (owner, name) in [
            (first_owner.clone(), "first"),
            (second_owner.clone(), "second"),
            (login_owner.clone(), "login"),
        ] {
            assert!(matches!(
                daemon.handle_as(
                    wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request_named(name)) },
                    owner,
                ),
                wire::Response::Furnished(_)
            ));
        }

        let first = daemon.subscribe(first_owner.clone()).unwrap();
        let second = daemon.subscribe(second_owner.clone()).unwrap();
        let login = daemon.subscribe(login_owner.clone()).unwrap();
        for (subscription, expected) in [(&first, "first"), (&second, "second"), (&login, "login")] {
            assert!(matches!(
                &subscription.initial.response,
                wire::Response::Subscribed(values)
                    if values.len() == 1 && values[0].workspace == workspace_named(expected)
            ));
        }

        assert!(matches!(
            daemon.handle_as(
                wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request_named("first")) },
                first_owner.clone(),
            ),
            wire::Response::Furnished(_)
        ));
        assert!(matches!(
            daemon.drain(first.id).as_deref(),
            Some([Event { response: wire::Response::Changed(_), .. }])
        ));
        assert!(daemon.drain(second.id).unwrap().is_empty());
        assert!(daemon.drain(login.id).unwrap().is_empty());

        let first_failure = wire::Response::CleanupFailed(wire::CleanupFailure {
            workspace: workspace_named("first"),
            generation_id: wire::GenerationId("sha256:first".into()),
            publication_revision: wire::PublicationRevision(1),
            file_failure: wire::FileFailure {
                workspace: workspace_named("first"),
                file_operation: wire::FileOperation::Cleaning,
                file_evidence: wire::FileEvidence::PermissionDenied,
            },
        });
        daemon.event(first_owner.clone(), first_failure);
        for revision in 2..20 {
            let mut other = daemon.state(&second_owner)[0].clone();
            other.publication_revision = wire::PublicationRevision(revision);
            daemon.event(second_owner.clone(), wire::Response::Changed(other));
        }
        assert!(matches!(
            daemon.drain(first.id).as_deref(),
            Some([Event { response: wire::Response::CleanupFailed(_), .. }])
        ));
        assert_eq!(daemon.drain(login.id).unwrap(), Vec::<Event>::new());

        let mut first_changed = daemon.state(&first_owner)[0].clone();
        first_changed.publication_revision = wire::PublicationRevision(20);
        daemon.event(first_owner.clone(), wire::Response::Changed(first_changed));
        assert!(matches!(
            daemon.drain(first.id).as_deref(),
            Some([Event { response: wire::Response::Changed(_), .. }])
        ));

        daemon.unsubscribe(second.id);
        let reconnected = daemon.subscribe(second_owner).unwrap();
        assert!(matches!(
            reconnected.initial.response,
            wire::Response::Subscribed(ref values)
                if values.len() == 1 && values[0].workspace == workspace_named("second")
        ));
    }

    #[test]
    fn cleanup_refusal_is_published_only_for_the_attempted_workspace_and_operation() {
        let mut image = ready_memory();
        image.configuration.ordinary_peers.as_mut().unwrap().0.push(translator::PeerBinding {
            peer_id: translator::PeerId("other".into()),
            peer_identity: translator::PeerIdentity {
                user_id: translator::UserId(1000),
                process_id: translator::ProcessId(55),
            },
        });
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let mut daemon = Daemon::start(
            bootstrap(),
            effects,
            FakeMemory { image: Some(image), ..Default::default() },
        )
        .unwrap();
        register(&mut daemon);
        let first_owner = peer_owner("client");
        let second_owner = peer_owner("other");
        for (owner, name) in [(first_owner.clone(), "first"), (second_owner, "second")] {
            assert!(matches!(
                daemon.handle_as(
                    wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request_named(name)) },
                    owner,
                ),
                wire::Response::Furnished(_)
            ));
        }
        let furnished = daemon.workspace(&workspace_named("first")).unwrap().successful.clone().unwrap();
        let maintenance = CleanupMaintenance {
            workspace: workspace_named("first"),
            generation_id: furnished.generation_id,
            publication_revision: furnished.publication_revision,
        };
        daemon.memory.cleanup_maintenance.push(maintenance.clone());
        let subscription = daemon.subscribe(first_owner).unwrap();

        daemon.effects.replies.push_back(emission::Response::CleanupRefused(wire::FileFailure {
            workspace: workspace_named("second"),
            file_operation: wire::FileOperation::Cleaning,
            file_evidence: wire::FileEvidence::PermissionDenied,
        }));
        daemon.run_cleanup(maintenance.clone());
        assert!(daemon.drain(subscription.id).unwrap().is_empty());
        assert!(daemon.memory.cleanup_maintenance.contains(&maintenance));

        daemon.effects.replies.push_back(emission::Response::CleanupRefused(wire::FileFailure {
            workspace: workspace_named("first"),
            file_operation: wire::FileOperation::Staging,
            file_evidence: wire::FileEvidence::PermissionDenied,
        }));
        daemon.run_cleanup(maintenance.clone());
        assert!(daemon.drain(subscription.id).unwrap().is_empty());
        assert!(daemon.memory.cleanup_maintenance.contains(&maintenance));

        daemon.effects.replies.push_back(emission::Response::CleanupRefused(wire::FileFailure {
            workspace: workspace_named("first"),
            file_operation: wire::FileOperation::Cleaning,
            file_evidence: wire::FileEvidence::PermissionDenied,
        }));
        daemon.run_cleanup(maintenance.clone());
        assert!(matches!(
            daemon.drain(subscription.id).as_deref(),
            Some([Event {
                response: wire::Response::CleanupFailed(wire::CleanupFailure {
                    workspace,
                    file_failure: wire::FileFailure { workspace: failed_workspace, file_operation: wire::FileOperation::Cleaning, .. },
                    ..
                }),
                ..
            }]) if workspace == &workspace_named("first") && failed_workspace == &workspace_named("first")
        ));
        assert!(daemon.memory.cleanup_maintenance.contains(&maintenance));
        let clean_requests: Vec<_> = daemon.effects.calls.iter().filter_map(|query| match query {
            emission::Query::Clean(request) => Some(request),
            _ => None,
        }).collect();
        assert_eq!(clean_requests.len(), 3);
        assert!(clean_requests.iter().all(|request| request.workspace == workspace_named("first")));
    }

    #[test]
    fn failed_refurnish_keeps_prior_success_and_emits_no_success() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let mut daemon = Daemon::start(
            bootstrap(),
            effects,
            FakeMemory { image: Some(ready_memory()), ..Default::default() },
        )
        .unwrap();
        register(&mut daemon);
        assert!(matches!(
            daemon.handle(wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request()) }),
            wire::Response::Furnished(_)
        ));
        let subscription = daemon.subscribe(wire::Owner::Login(caller().user_id.clone())).unwrap();
        daemon.effects.replies.push_back(emission::Response::Snapshotted(snapshot("two", "two")));
        daemon.effects.replies.push_back(emission::Response::EmissionRefused(
            emission::EmissionRefusal::Publication(emission::PublicationRefusal {
                workspace: workspace(),
                publication_operation: emission::PublicationOperation::StagingGeneration,
                publication_evidence: emission::PublicationEvidence::PermissionDenied,
            }),
        ));
        daemon.effects.replies.push_back(emission::Response::Inspected(
            emission::PublicationInspection {
                workspace: workspace(),
                publication_identity_option: None,
                publication_manifest_option: None,
            },
        ));
        daemon.effects.replies.push_back(emission::Response::EmissionRefused(
            emission::EmissionRefusal::Publication(emission::PublicationRefusal {
                workspace: workspace(),
                publication_operation: emission::PublicationOperation::StagingGeneration,
                publication_evidence: emission::PublicationEvidence::PermissionDenied,
            }),
        ));
        daemon.poll_once();
        let events = daemon.drain(subscription.id).unwrap();
        assert!(!events.is_empty());
        assert!(events.iter().all(|event| matches!(event.response, wire::Response::RefurnishFailed(_))));
        assert!(daemon.memory.workspaces[0].pending.is_some());
        assert!(daemon.memory.workspaces[0].successful.is_some());
        let reconnect = daemon.subscribe(wire::Owner::Login(caller().user_id)).unwrap_err();
        assert!(matches!(reconnect, wire::Refusal::SubscriberCapacityReached(1)));
        daemon.unsubscribe(subscription.id);
        let reconnect = daemon.subscribe(wire::Owner::Login(translator::UserId(1000))).unwrap();
        assert!(matches!(reconnect.initial.response, wire::Response::Subscribed(ref values) if values.len() == 1));
    }

    #[test]
    fn unrelated_repository_retry_failure_exposes_only_workspace_and_category() {
        let mut image = ready_memory();
        image.configuration.subscriber_capacity = Some(2);
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let mut daemon = Daemon::start(
            bootstrap(),
            effects,
            FakeMemory { image: Some(image), ..Default::default() },
        )
        .unwrap();
        register(&mut daemon);
        daemon.effects.replies.extend([
            emission::Response::Snapshotted(other_snapshot("other-one", "same")),
            emission::Response::Snapshotted(other_snapshot("other-one", "same")),
        ]);
        assert!(matches!(
            daemon.handle(wire::Call {
                caller: caller(),
                query: wire::Query::Register(other_repository()),
            }),
            wire::Response::Registered(_)
        ));
        assert!(matches!(
            daemon.handle(wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request()) }),
            wire::Response::Furnished(_)
        ));
        let owner = wire::Owner::Login(caller().user_id.clone());
        let owned = daemon.subscribe(owner).unwrap();
        let other = daemon.subscribe(peer_owner("client")).unwrap();

        daemon.effects.replies.extend([
            emission::Response::Snapshotted(snapshot("one", "one")),
            emission::Response::Snapshotted(other_snapshot("other-two", "same")),
            emission::Response::EmissionRefused(emission::EmissionRefusal::Publication(
                emission::PublicationRefusal {
                    workspace: workspace(),
                    publication_operation: emission::PublicationOperation::StagingGeneration,
                    publication_evidence: emission::PublicationEvidence::PermissionDenied,
                },
            )),
            emission::Response::Inspected(emission::PublicationInspection {
                workspace: workspace(),
                publication_identity_option: None,
                publication_manifest_option: None,
            }),
        ]);
        daemon.poll_once();
        let failures = daemon.drain(owned.id).unwrap();
        let expected_workspace = workspace();
        assert!(!failures.is_empty());
        assert!(failures.iter().all(|event| matches!(
            event.response,
            wire::Response::RefurnishFailed(wire::RefurnishFailure {
                ref workspace,
                refusal_kind: wire::RefusalKind::File,
            }) if workspace == &expected_workspace
        )));
        assert!(daemon.drain(other.id).unwrap().is_empty());
    }

    #[test]
    fn refusal_categories_discard_nested_diagnostics() {
        let repository = repository();
        let module_name = module();
        let source_evidence = [
            wire::SourceEvidence::NotRepository,
            wire::SourceEvidence::MainUnavailable,
            wire::SourceEvidence::RevisionUnavailable,
            wire::SourceEvidence::InvalidModule(wire::InvalidModule {
                module_name: module_name.clone(),
                relative_path: wire::RelativePath("vision/nexus.md".into()),
                invalid_placement: wire::InvalidPlacement::Unknown("secret-placement".into()),
            }),
            wire::SourceEvidence::Ambiguous(wire::Ambiguous_Data {
                module_name,
                repository_vector: vec![repository.clone(), other_repository()],
            }),
            wire::SourceEvidence::Translator(wire::Diagnostic {
                executable_path: wire::ExecutablePath("/private/git".into()),
                exit_status_option: Some(wire::ExitStatus(7)),
                string: "private diagnostic".into(),
            }),
        ];
        for source_evidence in source_evidence {
            let refusal = wire::Refusal::Source(wire::SourceFailure {
                repository: repository.clone(),
                source_operation: wire::SourceOperation::Watching,
                source_evidence,
            });
            assert_eq!(refusal_kind(&refusal), wire::RefusalKind::Source);
        }
        let caller = caller();
        let remaining = [
            (
                wire::Refusal::File(wire::FileFailure {
                    workspace: workspace(),
                    file_operation: wire::FileOperation::Staging,
                    file_evidence: wire::FileEvidence::PermissionDenied,
                }),
                wire::RefusalKind::File,
            ),
            (memory_refusal(wire::MemoryOperation::Recovering), wire::RefusalKind::Memory),
            (wire::Refusal::NotAuthorized(caller), wire::RefusalKind::NotAuthorized),
            (wire::Refusal::SubscriberCapacityReached(4), wire::RefusalKind::SubscriberCapacityReached),
            (wire::Refusal::NotReady, wire::RefusalKind::NotReady),
        ];
        for (refusal, expected) in remaining {
            assert_eq!(refusal_kind(&refusal), expected);
        }
    }

    #[test]
    fn cross_owner_cannot_replace_workspace() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            ..Default::default()
        };
        let mut image = ready_memory();
        image.configuration.ordinary_peers.as_mut().unwrap().0.push(translator::PeerBinding {
            peer_id: translator::PeerId("other".into()),
            peer_identity: translator::PeerIdentity {
                user_id: translator::UserId(1000),
                process_id: translator::ProcessId(55),
            },
        });
        let mut daemon = Daemon::start(
            bootstrap(),
            effects,
            FakeMemory { image: Some(image), ..Default::default() },
        )
        .unwrap();
        register(&mut daemon);
        assert!(matches!(
            daemon.handle_as(
                wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request()) },
                peer_owner("client"),
            ),
            wire::Response::Furnished(_)
        ));
        let response = daemon.handle_as(
            wire::Call { caller: caller(), query: wire::Query::Furnish(furnish_request()) },
            peer_owner("other"),
        );
        assert!(matches!(response, wire::Response::Refused(wire::Refusal::NotAuthorized(_))));
    }

    #[test]
    fn cleanup_failure_is_separate_and_retried_under_current_policy() {
        let effects = FakeEffects {
            replies: [
                emission::Response::Snapshotted(snapshot("one", "one")),
                emission::Response::Snapshotted(snapshot("one", "one")),
            ]
            .into(),
            publish_from_query: true,
            cleanup_failures: 2,
            ..Default::default()
        };
        let mut image = ready_memory();
        image.configuration.retention =
            meta::RetentionPolicy::KeepPrevious(meta::PositiveInteger(1));
        let memory = FakeMemory {
            image: Some(image),
            ..Default::default()
        };
        let mut daemon = Daemon::start(bootstrap(), effects, memory).unwrap();
        register(&mut daemon);
        let response = daemon.handle(wire::Call {
            caller: caller(),
            query: wire::Query::Furnish(furnish_request()),
        });
        assert!(matches!(response, wire::Response::Furnished(_)));
        let response = daemon.handle(wire::Call {
            caller: caller(),
            query: wire::Query::Furnish(furnish_request()),
        });
        assert!(matches!(response, wire::Response::Furnished(_)));
        assert!(daemon.memory.workspaces[0].successful.is_some());
        assert_eq!(daemon.memory.cleanup_maintenance.len(), 1);
        assert_eq!(daemon.memory.cleanup_maintenance[0].publication_revision, wire::PublicationRevision(1));
        assert!(daemon.memory.workspaces[0].pending.is_none());
        assert!(daemon
            .events
            .iter()
            .any(|event| matches!(event.response, wire::Response::CleanupFailed(_))));
        assert!(matches!(
            daemon.handle(wire::Call {
                caller: caller(),
                query: wire::Query::Furnish(furnish_request()),
            }),
            wire::Response::Refused(wire::Refusal::NotReady)
        ));

        daemon
            .effects
            .replies
            .push_back(emission::Response::Snapshotted(snapshot("one", "one")));
        daemon.poll_once();
        assert!(daemon.memory.cleanup_maintenance.is_empty());
        assert!(daemon.memory.workspaces[0].successful.is_some());
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BootstrapError {
    SocketCollision,
    InvalidAdmission,
    Memory(MemoryError),
    CorruptState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryError {
    Unavailable,
    Corrupt,
}

pub trait MemoryStore {
    fn load(&mut self) -> Result<Option<MemoryImage>, MemoryError>;
    fn store(&mut self, image: &MemoryImage) -> Result<(), MemoryError>;
}

pub struct FileMemory {
    path: PathBuf,
}

impl FileMemory {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl<E: EmissionPort> Daemon<E, FileMemory> {
    pub fn start_file(bootstrap: DaemonBootstrap, effects: E) -> Result<Self, BootstrapError> {
        let store = FileMemory::new(PathBuf::from(&bootstrap.nexus.memory_path.0));
        Self::start(bootstrap, effects, store)
    }
}

impl MemoryStore for FileMemory {
    fn load(&mut self) -> Result<Option<MemoryImage>, MemoryError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(MemoryError::Unavailable),
        };
        Signal::<MemoryImage>::from(bytes)
            .restore()
            .map(Some)
            .map_err(|_| MemoryError::Corrupt)
    }

    fn store(&mut self, image: &MemoryImage) -> Result<(), MemoryError> {
        let parent = self.path.parent().ok_or(MemoryError::Unavailable)?;
        fs::create_dir_all(parent).map_err(|_| MemoryError::Unavailable)?;
        let encoded = image.signalize().map_err(|_| MemoryError::Corrupt)?;
        let temporary = parent.join(format!(".belief-memory-{}.pending", std::process::id()));
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| MemoryError::Unavailable)?;
        file.write_all(encoded.bytes())
            .and_then(|()| file.sync_all())
            .map_err(|_| MemoryError::Unavailable)?;
        fs::rename(&temporary, &self.path).map_err(|_| MemoryError::Unavailable)?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| MemoryError::Unavailable)
    }
}

pub trait EmissionPort {
    fn emit(&mut self, attempt: emission::EmissionAttempt) -> emission::EmissionAttemptOutcome;
}

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Configuration {
    pub revision: meta::ConfigurationRevision,
    pub repository_polling: Option<meta::Timeout>,
    pub subscriber_capacity: Option<i64>,
    pub ordinary_peers: Option<wire::PeerList>,
    pub meta_peers: Option<wire::PeerList>,
    pub layouts: Vec<meta::HarnessLayout>,
    pub placements: Vec<meta::PlacementDefault>,
    pub retention: meta::RetentionPolicy,
    pub nexus_frame_capacity: Option<meta::ByteLimit>,
    pub nexus_transport_timeout: Option<meta::Timeout>,
    pub translator_sets: Vec<meta::Set>,
}

impl Configuration {
    fn empty() -> Self {
        Self {
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
        }
    }

    fn missing(&self) -> Vec<meta::SettingName> {
        let mut missing = Vec::new();
        if self.repository_polling.is_none() {
            missing.push(meta::SettingName::RepositoryPolling);
        }
        if self.subscriber_capacity.is_none() {
            missing.push(meta::SettingName::SubscriberCapacity);
        }
        if self.ordinary_peers.is_none() {
            missing.push(meta::SettingName::OrdinaryPeers);
        }
        if self.meta_peers.is_none() {
            missing.push(meta::SettingName::MetaPeers);
        }
        if self.nexus_frame_capacity.is_none() { missing.push(meta::SettingName::NexusFrameCapacity); }
        if self.nexus_transport_timeout.is_none() { missing.push(meta::SettingName::NexusTransportTimeout); }
        missing.extend(self.translator_missing());
        missing
    }

    fn translator_missing(&self) -> Vec<meta::SettingName> {
        let required = [
            meta::SettingName::NexusSocket, meta::SettingName::MetaNexusSocket,
            meta::SettingName::CallTimeout, meta::SettingName::OutputLimit,
            meta::SettingName::TextBudget, meta::SettingName::TranslatorFrameCapacity,
            meta::SettingName::TranslatorTransportTimeout, meta::SettingName::AllowedNexusPeers,
            meta::SettingName::GitExecutable,
        ];
        required.into_iter().filter(|name| !self.translator_sets.iter().any(|set| &setting_name(set) == name)).collect()
    }

    fn translator_configuration(&self) -> meta::TranslatorConfiguration {
        let missing = self.translator_missing();
        meta::TranslatorConfiguration {
            configuration_revision: self.revision.clone(),
            set_vector: self.translator_sets.clone(),
            readiness: if missing.is_empty() { meta::Readiness::Ready } else { meta::Readiness::Awaiting(missing) },
        }
    }

    fn readiness(&self) -> meta::Readiness {
        let missing = self.missing();
        if missing.is_empty() {
            meta::Readiness::Ready
        } else {
            meta::Readiness::Awaiting(missing)
        }
    }

    fn ready(&self) -> bool {
        self.missing().is_empty()
    }

    fn layout_for(&self, harness: &wire::Harness) -> Option<meta::Layout> {
        self.layouts.iter().find(|value| &value.harness == harness).map(|value| value.layout.clone())
    }

    fn placement_defaults(&self) -> HashMap<wire::Kind, wire::Placement> {
        self.placements.iter().map(|value| (value.kind.clone(), value.placement.clone())).collect()
    }
}

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StoredRepository {
    pub owner: wire::Owner,
    pub snapshot: emission::SourceSnapshot,
}

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StoredWorkspace {
    pub owner: wire::Owner,
    pub request: wire::FurnishRequest,
    pub configuration_revision: meta::ConfigurationRevision,
    pub layout: meta::Layout,
    pub placement_defaults: Vec<meta::PlacementDefault>,
    pub retention: meta::RetentionPolicy,
    pub successful: Option<wire::FurnishedWorkspace>,
    pub successful_history: Vec<SuccessfulGeneration>,
    pub successful_basis: Vec<wire::RepositoryState>,
    pub pending: Option<emission::Publication>,
}

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SuccessfulGeneration {
    pub generation_id: wire::GenerationId,
    pub publication_revision: wire::PublicationRevision,
}

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CleanupMaintenance {
    pub workspace: wire::Workspace,
    pub generation_id: wire::GenerationId,
    pub publication_revision: wire::PublicationRevision,
}

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MemoryImage {
    pub owner: wire::Owner,
    pub configuration: Configuration,
    pub repositories: Vec<StoredRepository>,
    pub workspaces: Vec<StoredWorkspace>,
    pub cleanup_maintenance: Vec<CleanupMaintenance>,
    pub next_event_sequence: i64,
}

impl MemoryImage {
    fn empty(owner: wire::Owner) -> Self {
        Self {
            owner,
            configuration: Configuration::empty(),
            repositories: Vec::new(),
            workspaces: Vec::new(),
            cleanup_maintenance: Vec::new(),
            next_event_sequence: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub sequence: i64,
    pub owner: wire::Owner,
    pub response: wire::Response,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subscription {
    pub id: i64,
    pub initial: Event,
}

#[derive(Clone, Debug)]
struct Subscriber {
    owner: wire::Owner,
    after: i64,
}

pub struct Daemon<E, M> {
    effects: E,
    store: M,
    memory: MemoryImage,
    core: BeliefCore,
    events: Vec<Event>,
    watch_failures: Vec<WatchFailure>,
    subscribers: HashMap<i64, Subscriber>,
    next_subscriber: i64,
}

impl<E: EmissionPort, M: MemoryStore> Daemon<E, M> {
    pub fn start(
        bootstrap: DaemonBootstrap,
        effects: E,
        mut store: M,
    ) -> Result<Self, BootstrapError> {
        validate_bootstrap(&bootstrap.nexus)?;
        let loaded = store.load().map_err(BootstrapError::Memory)?;
        let bootstrap_owner = bootstrap.nexus.bootstrap_owner.0.clone();
        let mut memory = loaded.clone().unwrap_or_else(|| MemoryImage::empty(bootstrap_owner));
        if loaded.is_none() {
            store.store(&memory).map_err(BootstrapError::Memory)?;
        }
        let core = rebuild_core(&memory).map_err(|_| BootstrapError::CorruptState)?;
        let mut daemon = Self {
            effects,
            store,
            memory,
            core,
            events: Vec::new(),
            watch_failures: Vec::new(),
            subscribers: HashMap::new(),
            next_subscriber: 1,
        };
        daemon.recover_pending();
        daemon.reconcile_startup();
        Ok(daemon)
    }

    fn reconcile_startup(&mut self) {
        let workspaces = self.core.retry_workspaces();
        for workspace in workspaces {
            let Some(stored) = self.workspace(&workspace).cloned() else {
                continue;
            };
            if stored.pending.is_some() {
                continue;
            }
            let owner = stored.owner.clone();
            if let wire::Response::Refused(refusal) = self.furnish(
                owner.clone(),
                translator::Caller {
                    user_id: translator::UserId(0),
                    process_id: translator::ProcessId(0),
                },
                stored.request,
            ) {
                self.event(owner, wire::Response::RefurnishFailed(wire::RefurnishFailure {
                    workspace,
                    refusal_kind: refusal_kind(&refusal),
                }));
            }
        }
    }

    fn emit_effect(&mut self, query: emission::Query) -> emission::Response {
        let revision = self.memory.configuration.revision.clone();
        let attempt = emission::EmissionAttempt {
            configuration_revision: revision.clone(),
            translator_configuration: self.memory.configuration.translator_configuration(),
            query: query.clone(),
        };
        let outcome = self.effects.emit(attempt);
        if outcome.configuration_revision == revision { outcome.response } else { emission_mismatch(&query) }
    }

    pub fn handle(&mut self, call: wire::Call) -> wire::Response {
        let owner = wire::Owner::Login(call.caller.user_id.clone());
        self.handle_as(call, owner)
    }

    pub fn handle_as(&mut self, call: wire::Call, owner: wire::Owner) -> wire::Response {
        if !self.owner_allowed(&owner) {
            return wire::Response::Refused(wire::Refusal::NotAuthorized(call.caller));
        }
        if !self.memory.configuration.ready() {
            return wire::Response::Refused(wire::Refusal::NotReady);
        }
        match call.query {
            wire::Query::Register(repository) => self.register(owner, call.caller, repository),
            wire::Query::Resolve(names) => {
                if self
                    .memory
                    .repositories
                    .iter()
                    .any(|repository| !same_owner(&repository.owner, &owner))
                {
                    wire::Response::Refused(wire::Refusal::NotAuthorized(call.caller))
                } else {
                    self.core
                        .resolve(&names)
                        .map(wire::Response::Resolved)
                        .unwrap_or_else(|error| wire::Response::Refused(resolve_refusal(error)))
                }
            }
            wire::Query::Furnish(request) => self.furnish(owner, call.caller, request),
            wire::Query::Subscribe => self
                .subscribe(owner)
                .map(|subscription| subscription.initial.response)
                .unwrap_or_else(wire::Response::Refused),
        }
    }

    pub fn open_subscription(&mut self, call: wire::Call) -> Result<Subscription, wire::Response> {
        let owner = wire::Owner::Login(call.caller.user_id.clone());
        self.open_subscription_as(call, owner)
    }

    pub fn open_subscription_as(&mut self, call: wire::Call, owner: wire::Owner) -> Result<Subscription, wire::Response> {
        if !self.owner_allowed(&owner) {
            return Err(wire::Response::Refused(wire::Refusal::NotAuthorized(
                call.caller,
            )));
        }
        if !self.memory.configuration.ready() {
            return Err(wire::Response::Refused(wire::Refusal::NotReady));
        }
        if !matches!(call.query, wire::Query::Subscribe) {
            return Err(wire::Response::Refused(wire::Refusal::NotReady));
        }
        self.subscribe(owner).map_err(wire::Response::Refused)
    }

    pub fn polling_timeout(&self) -> Option<meta::Timeout> {
        self.memory.configuration.repository_polling.clone()
    }

    pub fn configure(&mut self, call: meta::MetaCall) -> meta::Response {
        if self.memory.owner != wire::Owner::Login(call.caller.user_id.clone()) {
            return meta::Response::ConfigureRefused(meta::ConfigurationRefusal::NotAuthorized(
                call.caller,
            ));
        }
        let meta::Query::Configure(request) = call.query;
        if request.configuration_revision_option.as_ref().is_some_and(|expected|
            expected != &self.memory.configuration.revision)
        {
            return meta::Response::ConfigureRefused(meta::ConfigurationRefusal::StaleRevision(
                meta::StaleRevision {
                    configuration_revision_option: request.configuration_revision_option,
                    configuration_revision: self.memory.configuration.revision.clone(),
                },
            ));
        }
        let set = request.set.clone();
        let invalid = match &set {
            meta::Set::RepositoryPolling(value) if value.0 <= 0 => Some("must be positive"),
            meta::Set::SubscriberCapacity(value) if *value <= 0 => Some("must be positive"),
            meta::Set::GenerationRetention(meta::RetentionPolicy::KeepPrevious(value))
                if value.0 <= 0 =>
            {
                Some("must be positive")
            }
            _ => None,
        };
        if let Some(message) = invalid {
            return meta::Response::ConfigureRefused(meta::ConfigurationRefusal::Invalid(
                meta::Invalid {
                    keeper: meta::Keeper::Nexus,
                    setting_name: setting_name(&set),
                    string: message.into(),
                },
            ));
        }
        let Some(next_revision) = self.memory.configuration.revision.0.checked_add(1) else {
            return meta::Response::ConfigureRefused(meta::ConfigurationRefusal::Unavailable);
        };
        let previous = self.memory.configuration.clone();
        match &set {
            meta::Set::RepositoryPolling(value) => {
                self.memory.configuration.repository_polling = Some(value.clone())
            }
            meta::Set::SubscriberCapacity(value) => {
                self.memory.configuration.subscriber_capacity = Some(*value)
            }
            meta::Set::OrdinaryPeers(value) => {
                self.memory.configuration.ordinary_peers = Some(value.clone())
            }
            meta::Set::MetaPeers(value) => {
                self.memory.configuration.meta_peers = Some(value.clone())
            }
            meta::Set::Layout(value) => {
                self.memory.configuration.layouts.retain(|current| current.harness != value.harness);
                self.memory.configuration.layouts.push(value.clone());
            }
            meta::Set::Placement(value) => {
                self.memory.configuration.placements.retain(|current| current.kind != value.kind);
                self.memory.configuration.placements.push(value.clone());
            }
            meta::Set::GenerationRetention(value) => {
                self.memory.configuration.retention = value.clone()
            }
            meta::Set::NexusFrameCapacity(value) => self.memory.configuration.nexus_frame_capacity = Some(value.clone()),
            meta::Set::NexusTransportTimeout(value) => self.memory.configuration.nexus_transport_timeout = Some(value.clone()),
            _ if setting_owner(&set) == meta::Keeper::Translator => {
                let name = setting_name(&set);
                self.memory.configuration.translator_sets.retain(|current| setting_name(current) != name);
                self.memory.configuration.translator_sets.push(set.clone());
            }
            _ => unreachable!("all Nexus-owned settings are handled above"),
        }
        self.memory.configuration.revision.0 = next_revision;
        if self.persist().is_err() {
            self.memory.configuration = previous;
            return meta::Response::ConfigureRefused(meta::ConfigurationRefusal::Unavailable);
        }
        let keeper = setting_owner(&set);
        meta::Response::Configured(meta::ConfiguredState {
            set,
            keeper,
            configuration_revision: self.memory.configuration.revision.clone(),
            readiness: self.memory.configuration.readiness(),
        })
    }

    pub fn meta_transport(&mut self, input: meta::MetaTransportInput) -> meta::MetaTransportOutput {
        match input {
            meta::MetaTransportInput::Call(call) => meta::MetaTransportOutput::Answered(self.configure(call)),
            meta::MetaTransportInput::RestoreTranslatorConfiguration => {
                meta::MetaTransportOutput::Restored(self.memory.configuration.translator_configuration())
            }
        }
    }

    pub fn recover_pending(&mut self) -> Vec<wire::Response> {
        let workspaces: Vec<_> = self
            .memory
            .workspaces
            .iter()
            .filter(|record| record.pending.is_some())
            .map(|record| record.request.workspace.clone())
            .collect();
        workspaces
            .into_iter()
            .filter_map(|workspace| self.recover_one(&workspace))
            .collect()
    }

    pub fn poll_once(&mut self) -> Vec<Event> {
        let offset = self.events.len();
        self.watch_failures.clear();
        let repositories: Vec<_> = self
            .memory
            .repositories
            .iter()
            .map(|stored| {
                (
                    stored.snapshot.repository.clone(),
                    stored.snapshot.source_revision.clone(),
                )
            })
            .collect();
        for (repository, old_revision) in repositories {
            match self.emit_effect(emission::Query::Snapshot(repository.clone()))
            {
                emission::Response::Snapshotted(snapshot)
                    if snapshot.repository != repository => self
                        .watch_failures
                        .push(watch_unavailable(repository, Some(old_revision))),
                emission::Response::Snapshotted(snapshot)
                    if snapshot.source_revision == old_revision => {}
                emission::Response::Snapshotted(snapshot) => self.apply_snapshot(snapshot),
                emission::Response::EmissionRefused(emission::EmissionRefusal::Source(source)) => {
                    let failure = map_source(source, wire::SourceOperation::Watching);
                    self.watch_failures.push(WatchFailure {
                        repository,
                        source_revision_option: Some(old_revision),
                        source_failure: failure,
                    });
                }
                _ => self.watch_failures.push(watch_unavailable(
                    repository,
                    Some(old_revision),
                )),
            }
        }
        self.retry_pending_events();
        self.retry_cleanup_maintenance();
        self.maintain_workspaces();
        self.events[offset..].to_vec()
    }

    pub fn drain(&mut self, id: i64) -> Option<Vec<Event>> {
        let subscriber = self.subscribers.get_mut(&id)?;
        let events: Vec<_> = self
            .events
            .iter()
            .filter(|event| event.sequence > subscriber.after)
            .filter(|event| same_owner(&event.owner, &subscriber.owner))
            .cloned()
            .collect();
        subscriber.after = self.memory.next_event_sequence - 1;
        self.prune_events();
        Some(events)
    }

    pub fn unsubscribe(&mut self, id: i64) {
        self.subscribers.remove(&id);
        self.prune_events();
    }

    pub fn subscriber_owner(&self, id: i64) -> Option<&wire::Owner> {
        Some(&self.subscribers.get(&id)?.owner)
    }

    fn prune_events(&mut self) {
        let after = self
            .subscribers
            .values()
            .map(|subscriber| subscriber.after)
            .min()
            .unwrap_or(self.memory.next_event_sequence - 1);
        self.events.retain(|event| event.sequence > after);
    }

    fn register(&mut self, owner: wire::Owner, caller: translator::Caller, repository: wire::Repository) -> wire::Response {
        if self
            .memory
            .repositories
            .iter()
            .any(|stored| !same_owner(&stored.owner, &owner))
        {
            return wire::Response::Refused(wire::Refusal::NotAuthorized(caller));
        }
        if self
            .repository(&repository)
            .is_some_and(|stored| !same_owner(&stored.owner, &owner))
        {
            return wire::Response::Refused(wire::Refusal::NotAuthorized(caller));
        }
        let first = match self.emit_effect(emission::Query::Snapshot(repository.clone()))
        {
            emission::Response::Snapshotted(snapshot) if snapshot.repository == repository => snapshot,
            emission::Response::EmissionRefused(refusal) => {
                return wire::Response::Refused(map_emission(refusal));
            }
            _ => return wire::Response::Refused(wire::Refusal::NotReady),
        };
        if self.install_repository(owner, first.clone()).is_err() {
            return wire::Response::Refused(memory_refusal(wire::MemoryOperation::Recording));
        }
        // Immediate next observation closes the register/watch handoff window.
        if let emission::Response::Snapshotted(next) = self.emit_effect(emission::Query::Snapshot(repository.clone()))
        {
            if next.repository == repository && next.source_revision != first.source_revision {
                self.apply_snapshot(next);
            }
        }
        let revision = self
            .repository(&repository)
            .expect("registered repository")
            .snapshot
            .source_revision
            .clone();
        wire::Response::Registered(wire::RepositoryState {
            repository,
            source_revision: revision,
        })
    }

    fn install_repository(
        &mut self,
        owner: wire::Owner,
        snapshot: emission::SourceSnapshot,
    ) -> Result<(), ()> {
        let previous = self.memory.clone();
        self.core.observe(snapshot.clone().into()).map_err(|_| ())?;
        if let Some(stored) = self.repository_mut(&snapshot.repository) {
            stored.snapshot = snapshot;
        } else {
            self.memory
                .repositories
                .push(StoredRepository { owner, snapshot });
        }
        self.sort();
        if self.persist().is_err() {
            self.memory = previous;
            self.core = rebuild_core(&self.memory).map_err(|_| ())?;
            return Err(());
        }
        Ok(())
    }

    fn furnish(&mut self, owner: wire::Owner, caller: translator::Caller, request: wire::FurnishRequest) -> wire::Response {
        if self
            .workspace(&request.workspace)
            .is_some_and(|stored| !same_owner(&stored.owner, &owner))
        {
            return wire::Response::Refused(wire::Refusal::NotAuthorized(caller));
        }
        if self
            .workspace(&request.workspace)
            .is_some_and(|stored| stored.pending.is_some())
        {
            return wire::Response::Refused(wire::Refusal::NotReady);
        }
        if self
            .memory
            .cleanup_maintenance
            .iter()
            .any(|maintenance| maintenance.workspace == request.workspace)
        {
            self.retry_cleanup_workspace(&request.workspace);
            if self
                .memory
                .cleanup_maintenance
                .iter()
                .any(|maintenance| maintenance.workspace == request.workspace)
            {
                return wire::Response::Refused(wire::Refusal::NotReady);
            }
        }
        let previous = self.memory.clone();
        let revision = self.memory.configuration.revision.clone();
        let Some(layout) = self.memory.configuration.layout_for(&request.harness) else { return wire::Response::Refused(wire::Refusal::NotReady) };
        let placement_defaults = self.memory.configuration.placements.clone();
        let defaults = self.memory.configuration.placement_defaults();
        let retention = self.memory.configuration.retention.clone();
        let plan = match self.core.prepare_furnish_with_defaults(request.clone(), &defaults) {
            Ok(plan) => plan,
            Err(error) => return wire::Response::Refused(resolve_refusal(error)),
        };
        let publication = match self.core.begin_publication(
            &plan,
            PublicationContext {
                owner: owner.clone(),
                configuration_revision: revision.clone(),
                layout: layout.clone(),
                retention_policy: retention.clone(),
            },
        ) {
            Ok(publication) => publication,
            Err(_) => return wire::Response::Refused(file_refusal(request.workspace.clone())),
        };
        if let Some(stored) = self.workspace_mut(&request.workspace) {
            stored.owner = owner;
            stored.request = request.clone();
            stored.configuration_revision = revision;
            stored.layout = layout;
            stored.placement_defaults = placement_defaults;
            stored.retention = retention;
            stored.pending = Some(publication.clone());
        } else {
            self.memory.workspaces.push(StoredWorkspace {
                owner,
                request: request.clone(),
                configuration_revision: revision,
                layout,
                placement_defaults,
                retention,
                successful: None,
                successful_history: Vec::new(),
                successful_basis: Vec::new(),
                pending: Some(publication.clone()),
            });
        }
        self.sort();
        if self.persist().is_err() {
            self.memory = previous;
            if let Ok(core) = rebuild_core(&self.memory) {
                self.core = core;
            }
            return wire::Response::Refused(memory_refusal(
                wire::MemoryOperation::BeginningPublication,
            ));
        }
        self.publish(publication)
    }

    fn publish(&mut self, publication: emission::Publication) -> wire::Response {
        match self.emit_effect(emission::Query::Publish(publication)) {
            emission::Response::Published(state) => {
                let furnished = match self.core.complete_publication(&state) {
                    Ok(furnished) => furnished,
                    Err(_) => return wire::Response::Refused(file_refusal(state.workspace)),
                };
                let before = self.workspace(&state.workspace).cloned();
                let owner = before.as_ref().map(|stored| stored.owner.clone());
                if let Some(stored) = self.workspace_mut(&state.workspace) {
                    if let Some(previous) = stored.successful.take() {
                        stored.successful_history.push(SuccessfulGeneration {
                            generation_id: previous.generation_id,
                            publication_revision: previous.publication_revision,
                        });
                    }
                    stored.successful = Some(furnished.clone());
                    stored.successful_basis = stored
                        .pending
                        .as_ref()
                        .map(|pending| pending.repository_states.0.clone())
                        .unwrap_or_default();
                    stored.pending = None;
                }
                if let Err(error) = self.persist() {
                    if let Some(before) = before {
                        if let Some(stored) = self.workspace_mut(&state.workspace) {
                            *stored = before;
                        }
                    }
                    if let Ok(core) = rebuild_core(&self.memory) {
                        self.core = core;
                    }
                    let answer =
                        wire::Response::MemoryAfterPublication(wire::MemoryAfterPublication {
                            workspace: state.workspace,
                            generation_id: state.publication_identity.generation_id,
                            publication_revision: state.publication_identity.publication_revision,
                            memory_failure: memory_failure(
                                wire::MemoryOperation::CompletingPublication,
                                error,
                            ),
                        });
                    if let Some(owner) = owner {
                        self.event(owner, answer.clone());
                    }
                    return answer;
                }
                if let Some(owner) = owner {
                    self.event(owner, wire::Response::Changed(furnished.clone()));
                }
                self.clean_after_success(&furnished);
                wire::Response::Furnished(furnished)
            }
            emission::Response::EmissionRefused(refusal) => {
                wire::Response::Refused(map_emission(refusal))
            }
            _ => wire::Response::Refused(wire::Refusal::NotReady),
        }
    }

    fn clean_after_success(&mut self, furnished: &wire::FurnishedWorkspace) {
        if self
            .workspace(&furnished.workspace)
            .is_some_and(|stored| stored.pending.is_some())
        {
            return;
        }
        if self
            .memory
            .cleanup_maintenance
            .iter()
            .any(|maintenance| maintenance.workspace == furnished.workspace)
        {
            self.retry_cleanup_workspace(&furnished.workspace);
            return;
        }
        let Some(target) = self
            .cleanup_candidates(&furnished.workspace)
            .first()
            .cloned()
        else {
            return;
        };
        let maintenance = CleanupMaintenance {
            workspace: furnished.workspace.clone(),
            generation_id: target.generation_id,
            publication_revision: target.publication_revision,
        };
        self.memory.cleanup_maintenance.push(maintenance.clone());
        if self.persist().is_err() {
            self.memory
                .cleanup_maintenance
                .retain(|value| value != &maintenance);
            return;
        }
        self.run_cleanup(maintenance);
    }

    fn cleanup_candidates(&self, workspace: &wire::Workspace) -> Vec<SuccessfulGeneration> {
        let Some(stored) = self.workspace(workspace) else {
            return Vec::new();
        };
        let keep = match self.memory.configuration.retention {
            meta::RetentionPolicy::KeepAll => return Vec::new(),
            meta::RetentionPolicy::KeepPrevious(ref value) => value.0.max(1) as usize,
        };
        let total = stored.successful_history.len() + usize::from(stored.successful.is_some());
        let removable = total.saturating_sub(keep);
        let protected: Vec<_> = self
            .memory
            .workspaces
            .iter()
            .filter_map(|record| record.pending.as_ref())
            .map(|pending| emission::PublicationIdentity {
                generation_id: pending.generation_id.clone(),
                publication_revision: pending.intended_publication.0.clone(),
            })
            .collect();
        stored
            .successful_history
            .iter()
            .take(removable)
            .filter(|generation| !protected.contains(&emission::PublicationIdentity {
                generation_id: generation.generation_id.clone(),
                publication_revision: generation.publication_revision.clone(),
            }))
            .cloned()
            .collect()
    }

    fn run_cleanup(&mut self, maintenance: CleanupMaintenance) {
        let Some(stored) = self.workspace(&maintenance.workspace).cloned() else {
            return;
        };
        let owner = stored.owner.clone();
        let request = emission::CleanupRequest {
            workspace: maintenance.workspace.clone(),
            publication_identity: emission::PublicationIdentity {
                generation_id: maintenance.generation_id.clone(),
                publication_revision: maintenance.publication_revision.clone(),
            },
            layout: stored.layout,
        };
        match self.emit_effect(emission::Query::Clean(request)) {
            emission::Response::Cleaned(state)
                if state.workspace == maintenance.workspace
                    && state.publication_identity.generation_id == maintenance.generation_id
                    && state.publication_identity.publication_revision == maintenance.publication_revision =>
            {
                let previous = self.memory.clone();
                if let Some(stored) = self.workspace_mut(&maintenance.workspace) {
                    stored
                        .successful_history
                        .retain(|generation| generation.generation_id != maintenance.generation_id
                            || generation.publication_revision != maintenance.publication_revision);
                }
                self.memory
                    .cleanup_maintenance
                    .retain(|value| value != &maintenance);
                if self.persist().is_err() {
                    self.memory = previous;
                }
            }
            emission::Response::CleanupRefused(file_failure)
                if file_failure.workspace == maintenance.workspace
                    && file_failure.file_operation == wire::FileOperation::Cleaning =>
            {
                self.event(owner, wire::Response::CleanupFailed(wire::CleanupFailure {
                    workspace: maintenance.workspace,
                    generation_id: maintenance.generation_id,
                    publication_revision: maintenance.publication_revision,
                    file_failure,
                }));
            }
            _ => {}
        }
    }

    fn recover_one(&mut self, workspace: &wire::Workspace) -> Option<wire::Response> {
        let stored = self.workspace(workspace)?.clone();
        let response = self.emit_effect(emission::Query::InspectPublication(
            emission::InspectionRequest {
                workspace: workspace.clone(),
                publication_identity: emission::PublicationIdentity {
                    generation_id: stored.pending.as_ref()?.generation_id.clone(),
                    publication_revision: stored.pending.as_ref()?.intended_publication.0.clone(),
                },
                layout: stored.layout.clone(),
            },
        ));
        match response {
            emission::Response::Inspected(inspection) => {
                match self.core.recover_publication(&inspection) {
                    Ok(RecoveryOutcome::Finalized(furnished)) => {
                        let previous = self.memory.clone();
                        if let Some(record) = self.workspace_mut(workspace) {
                            if let Some(previous) = record.successful.take() {
                                record.successful_history.push(SuccessfulGeneration {
                                    generation_id: previous.generation_id,
                                    publication_revision: previous.publication_revision,
                                });
                            }
                            record.successful = Some(furnished.clone());
                            record.successful_basis = record
                                .pending
                                .as_ref()
                                .map(|pending| pending.repository_states.0.clone())
                                .unwrap_or_default();
                            record.pending = None;
                        }
                        if self.persist().is_err() {
                            self.memory = previous;
                            if let Ok(core) = rebuild_core(&self.memory) {
                                self.core = core;
                            }
                            let answer = wire::Response::MemoryAfterPublication(
                                wire::MemoryAfterPublication {
                                    workspace: workspace.clone(),
                                    generation_id: furnished.generation_id,
                                    publication_revision: furnished.publication_revision,
                                    memory_failure: memory_failure(
                                        wire::MemoryOperation::CompletingPublication,
                                        MemoryError::Unavailable,
                                    ),
                                },
                            );
                            self.event(stored.owner.clone(), answer.clone());
                            return Some(answer);
                        }
                        self.event(stored.owner.clone(), wire::Response::Changed(furnished.clone()));
                        self.clean_after_success(&furnished);
                        Some(wire::Response::Furnished(furnished))
                    }
                    Ok(RecoveryOutcome::Retry) => {
                        stored.pending.map(|pending| self.publish(pending))
                    }
                    Err(_) => Some(wire::Response::Refused(file_refusal(workspace.clone()))),
                }
            }
            emission::Response::EmissionRefused(refusal) => {
                Some(wire::Response::Refused(map_emission(refusal)))
            }
            _ => Some(wire::Response::Refused(wire::Refusal::NotReady)),
        }
    }

    fn apply_snapshot(&mut self, snapshot: emission::SourceSnapshot) {
        let previous = self.memory.clone();
        let repository = snapshot.repository.clone();
        let revision = snapshot.source_revision.clone();
        let change = match self.core.observe(snapshot.clone().into()) {
            Ok(change) => change,
            Err(_) => {
                self.watch_failures.push(watch_unavailable(
                    repository,
                    Some(revision),
                ));
                return;
            }
        };
        if let Some(stored) = self.repository_mut(&repository) {
            stored.snapshot = snapshot;
        }
        if self.persist().is_err() {
            self.memory = previous;
            if let Ok(core) = rebuild_core(&self.memory) {
                self.core = core;
            }
            return;
        }
        for workspace in change.retry_workspaces {
            let Some(stored) = self.workspace(&workspace).cloned() else {
                continue;
            };
            let owner = stored.owner.clone();
            match self.furnish(owner.clone(), translator::Caller { user_id: translator::UserId(0), process_id: translator::ProcessId(0) }, stored.request) {
                wire::Response::Furnished(_) => {}
                wire::Response::Refused(refusal) => {
                    self.event(owner, wire::Response::RefurnishFailed(wire::RefurnishFailure {
                        workspace,
                        refusal_kind: refusal_kind(&refusal),
                    }));
                }
                _ => {}
            }
        }
    }

    pub fn subscribe(&mut self, owner: wire::Owner) -> Result<Subscription, wire::Refusal> {
        let capacity = self.memory.configuration.subscriber_capacity.unwrap_or(0);
        if self.subscribers.len() as i64 >= capacity {
            return Err(wire::Refusal::SubscriberCapacityReached(capacity));
        }
        let id = self.next_subscriber;
        self.next_subscriber += 1;
        let initial = self.next_event(owner.clone(), wire::Response::Subscribed(self.state(&owner)));
        self.subscribers.insert(
            id,
            Subscriber {
                owner,
                after: initial.sequence,
            },
        );
        if self.persist().is_err() {
            self.subscribers.remove(&id);
            self.memory.next_event_sequence -= 1;
            return Err(memory_refusal(wire::MemoryOperation::Recording));
        }
        Ok(Subscription { id, initial })
    }

    fn state(&self, owner: &wire::Owner) -> Vec<wire::FurnishedWorkspace> {
        let mut furnished: Vec<_> = self
            .memory
            .workspaces
            .iter()
            .filter(|workspace| same_owner(&workspace.owner, owner))
            .filter_map(|workspace| workspace.successful.clone())
            .collect();
        furnished.sort_by_key(|value| value.workspace.0 .0.clone());
        furnished
    }

    fn event(&mut self, owner: wire::Owner, response: wire::Response) {
        let event = self.next_event(owner, response);
        if self.persist().is_ok() {
            self.events.push(event);
        } else {
            self.memory.next_event_sequence -= 1;
        }
    }

    fn next_event(&mut self, owner: wire::Owner, response: wire::Response) -> Event {
        let sequence = self.memory.next_event_sequence;
        self.memory.next_event_sequence += 1;
        Event { sequence, owner, response }
    }

    fn retry_pending_events(&mut self) {
        let workspaces: Vec<_> = self
            .memory
            .workspaces
            .iter()
            .filter(|record| record.pending.is_some())
            .map(|record| record.request.workspace.clone())
            .collect();
        for workspace in workspaces {
            let owner = self.workspace(&workspace).map(|record| record.owner.clone());
            if let Some(response) = self.recover_one(&workspace) {
                match (response, owner) {
                    (wire::Response::Refused(refusal), Some(owner)) => {
                        self.event(owner, wire::Response::RefurnishFailed(wire::RefurnishFailure {
                            workspace,
                            refusal_kind: refusal_kind(&refusal),
                        }));
                    }
                    _ => {}
                }
            }
        }
    }

    fn retry_cleanup_maintenance(&mut self) {
        let workspaces: Vec<_> = self
            .memory
            .cleanup_maintenance
            .iter()
            .map(|maintenance| maintenance.workspace.clone())
            .collect();
        for workspace in workspaces {
            self.retry_cleanup_workspace(&workspace);
        }
    }

    fn maintain_workspaces(&mut self) {
        let furnished: Vec<_> = self
            .memory
            .workspaces
            .iter()
            .filter(|record| record.pending.is_none())
            .filter_map(|record| record.successful.clone())
            .collect();
        for workspace in furnished {
            self.clean_after_success(&workspace);
        }
    }

    fn retry_cleanup_workspace(&mut self, workspace: &wire::Workspace) {
        if self
            .workspace(workspace)
            .is_some_and(|record| record.pending.is_some())
        {
            return;
        }
        let Some(maintenance) = self
            .memory
            .cleanup_maintenance
            .iter()
            .find(|maintenance| &maintenance.workspace == workspace)
            .cloned()
        else {
            return;
        };
        let eligible = self
            .cleanup_candidates(workspace)
            .iter()
            .any(|candidate| candidate.generation_id == maintenance.generation_id
                && candidate.publication_revision == maintenance.publication_revision);
        if eligible {
            self.run_cleanup(maintenance);
        } else {
            let previous = self.memory.clone();
            self.memory
                .cleanup_maintenance
                .retain(|value| value != &maintenance);
            if self.persist().is_err() {
                self.memory = previous;
            }
        }
    }

    fn persist(&mut self) -> Result<(), MemoryError> {
        self.store.store(&self.memory)
    }

    pub fn direct_owner(&self, caller: &translator::Caller) -> Option<wire::Owner> {
        self.memory.configuration.ordinary_peers.as_ref().and_then(|peers| peer_owner(peers, caller))
    }

    pub fn login_owner(&self, caller: &translator::Caller) -> Option<wire::Owner> {
        let owner = wire::Owner::Login(caller.user_id.clone());
        self.owner_allowed(&owner).then_some(owner)
    }

    fn owner_allowed(&self, owner: &wire::Owner) -> bool {
        match owner {
            wire::Owner::Login(_) => owner == &self.memory.owner,
            wire::Owner::Peer(peer_id) => self.memory.configuration.ordinary_peers.as_ref().is_some_and(|peers| peers.0.iter().any(|peer| &peer.peer_id == peer_id)),
        }
    }

    fn repository(&self, repository: &wire::Repository) -> Option<&StoredRepository> {
        self.memory
            .repositories
            .iter()
            .find(|stored| &stored.snapshot.repository == repository)
    }

    fn repository_mut(&mut self, repository: &wire::Repository) -> Option<&mut StoredRepository> {
        self.memory
            .repositories
            .iter_mut()
            .find(|stored| &stored.snapshot.repository == repository)
    }

    fn workspace(&self, workspace: &wire::Workspace) -> Option<&StoredWorkspace> {
        self.memory
            .workspaces
            .iter()
            .find(|stored| &stored.request.workspace == workspace)
    }

    fn workspace_mut(&mut self, workspace: &wire::Workspace) -> Option<&mut StoredWorkspace> {
        self.memory
            .workspaces
            .iter_mut()
            .find(|stored| &stored.request.workspace == workspace)
    }

    fn sort(&mut self) {
        self.memory
            .repositories
            .sort_by_key(|value| value.snapshot.repository.0 .0.clone());
        self.memory
            .workspaces
            .sort_by_key(|value| value.request.workspace.0 .0.clone());
    }
}
