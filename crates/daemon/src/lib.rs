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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonBootstrap {
    pub nexus: meta::NexusBootstrap,
    pub memory_file: PathBuf,
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
        let plan = core
            .prepare_furnish(workspace.request.clone())
            .map_err(|_| ())?;
        let context = || PublicationContext {
            owner: workspace.owner.clone(),
            configuration_revision: workspace.configuration_revision.clone(),
            layout: workspace.layout.clone(),
            retention_policy: workspace.retention.clone(),
        };
        if let Some(pending) = &workspace.pending {
            if &core.begin_publication(&plan, context()).map_err(|_| ())? != pending {
                return Err(());
            }
        }
    }
    Ok(core)
}

fn validate_bootstrap(value: &meta::NexusBootstrap) -> Result<(), BootstrapError> {
    if value.ordinary_socket.0 .0 == value.meta_socket.0 .0 {
        return Err(BootstrapError::SocketCollision);
    }
    if value.admission.byte_limit.0 <= 0 || value.admission.timeout.0 <= 0 {
        return Err(BootstrapError::InvalidAdmission);
    }
    Ok(())
}

fn paired(pair: &meta::PairedProcess, caller: &wire::Caller) -> bool {
    pair.user_id == caller.user_id
        && pair.group_id == caller.group_id
        && pair.process_id == caller.process_id
}

fn peer_allowed(peers: &wire::PeerList, caller: &wire::Caller) -> bool {
    peers.0.iter().any(|peer| {
        peer.user_id == caller.user_id
            && peer.group_id == caller.group_id
            && peer.executable_path == caller.executable_path
            && peer.contract_digest == caller.contract_digest
    })
}

fn same_owner(first: &wire::Caller, second: &wire::Caller) -> bool {
    first.user_id == second.user_id
        && first.group_id == second.group_id
        && first.executable_path == second.executable_path
        && first.contract_digest == second.contract_digest
}

fn empty_layout() -> meta::Layout {
    let path = wire::RelativePath(String::new());
    meta::Layout {
        owned_root: meta::OwnedRoot(path.clone()),
        generations_directory: meta::GenerationsDirectory(path.clone()),
        current_link: meta::CurrentLink(path.clone()),
        settings_file: meta::SettingsFile(path.clone()),
        instructions_setting: meta::InstructionsSetting(String::new()),
        instructions_file: meta::InstructionsFile(path.clone()),
        skills_directory: meta::SkillsDirectory(path),
    }
}

fn setting_name(set: &meta::Set) -> meta::SettingName {
    match set {
        meta::Set::RepositoryPolling(_) => meta::SettingName::RepositoryPolling,
        meta::Set::SubscriberCapacity(_) => meta::SettingName::SubscriberCapacity,
        meta::Set::OrdinaryPeers(_) => meta::SettingName::OrdinaryPeers,
        meta::Set::MetaPeers(_) => meta::SettingName::MetaPeers,
        meta::Set::Layout(value) => meta::SettingName::Layout(value.clone()),
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

fn cleanup_refusal(value: emission::CleanupRefusal) -> wire::Refusal {
    wire::Refusal::File(wire::FileFailure {
        workspace: value.workspace,
        file_operation: wire::FileOperation::Inspecting,
        file_evidence: wire::FileEvidence::PermissionDenied,
    })
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

fn watch_unavailable(
    repository: wire::Repository,
    revision: Option<wire::SourceRevision>,
) -> wire::WatchFailure {
    wire::WatchFailure {
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

    #[derive(Default)]
    struct FakeEffects {
        replies: VecDeque<emission::Response>,
        calls: Vec<emission::Query>,
        publish_from_query: bool,
    }

    impl EmissionPort for FakeEffects {
        fn emit(&mut self, query: emission::Query) -> emission::Response {
            self.calls.push(query.clone());
            if let Some(reply) = self.replies.pop_front() {
                return reply;
            }
            match query {
                emission::Query::Publish(publication) if self.publish_from_query => {
                    emission::Response::Published(emission::PublicationState {
                        workspace: publication.workspace,
                        generation_id: publication.generation_id,
                        publication_revision: publication.intended_publication.0,
                    })
                }
                emission::Query::Clean(request) => {
                    emission::Response::Cleaned(emission::CleanupState {
                        workspace: request.workspace,
                        removed_generations: emission::RemovedGenerations(Vec::new()),
                    })
                }
                _ => panic!("missing effect reply"),
            }
        }
    }

    #[derive(Default)]
    struct FakeMemory {
        image: Option<MemoryImage>,
        stores: usize,
        fail_at: Option<usize>,
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
            self.image = Some(image.clone());
            Ok(())
        }
    }

    fn caller() -> wire::Caller {
        wire::Caller {
            user_id: wire::UserId(1000),
            group_id: wire::GroupId(1000),
            process_id: wire::ProcessId(44),
            executable_path: wire::ExecutablePath("/nix/client".into()),
            contract_digest: wire::ContractDigest("contract".into()),
        }
    }

    fn bootstrap() -> DaemonBootstrap {
        DaemonBootstrap {
            nexus: meta::NexusBootstrap {
                ordinary_socket: meta::OrdinarySocket(meta::SocketPath("/run/ordinary".into())),
                meta_socket: meta::MetaSocket(meta::SocketPath("/run/meta".into())),
                admission: meta::Admission {
                    byte_limit: meta::ByteLimit(1_000_000),
                    timeout: meta::Timeout(5_000),
                },
                paired_process: meta::PairedProcess {
                    user_id: wire::UserId(1000),
                    group_id: wire::GroupId(1000),
                    process_id: wire::ProcessId(44),
                },
            },
            memory_file: PathBuf::from("/unused"),
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
        let peer = wire::PeerIdentity {
            peer_name: wire::PeerName("client".into()),
            user_id: wire::UserId(1000),
            group_id: wire::GroupId(1000),
            executable_path: wire::ExecutablePath("/nix/client".into()),
            contract_digest: wire::ContractDigest("contract".into()),
        };
        MemoryImage {
            configuration: Configuration {
                revision: meta::ConfigurationRevision(4),
                repository_polling: Some(meta::Timeout(100)),
                subscriber_capacity: Some(1),
                ordinary_peers: Some(wire::PeerList(vec![peer.clone()])),
                meta_peers: Some(wire::PeerList(vec![peer])),
                layout: Some(layout()),
                retention: meta::RetentionPolicy::KeepAll,
            },
            repositories: Vec::new(),
            workspaces: Vec::new(),
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

    fn workspace() -> wire::Workspace {
        wire::Workspace(wire::WorkspacePath("/work".into()))
    }

    fn snapshot(revision: &str, content: &str) -> emission::SourceSnapshot {
        emission::SourceSnapshot {
            repository: repository(),
            source_revision: wire::SourceRevision(revision.into()),
            module_source_vector: vec![emission::ModuleSource {
                module_name: module(),
                relative_path: wire::RelativePath("vision/nexus.md".into()),
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

    fn register(daemon: &mut Daemon<FakeEffects, FakeMemory>) {
        let response = daemon.handle(wire::Call {
            caller: caller(),
            query: wire::Query::Register(repository()),
        });
        assert!(matches!(response, wire::Response::Registered(_)));
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
            owned_pointer_option: Some(emission::OwnedPointer {
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
        let mut second = Daemon::start(bootstrap(), effects, memory).unwrap();
        assert!(matches!(
            second.recover_pending().as_slice(),
            [wire::Response::Furnished(_)]
        ));
        assert!(second.memory.workspaces[0].pending.is_none());
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
        let first = daemon.subscribe(caller()).unwrap();
        assert!(matches!(
            daemon.subscribe(caller()),
            Err(wire::Refusal::SubscriberCapacityReached(1))
        ));
        daemon.unsubscribe(first.id);
        assert!(daemon.subscribe(caller()).is_ok());
    }

    #[test]
    fn changed_watch_announces_observation_then_retry_publication_without_new_commit() {
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
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0].response,
            wire::Response::Changed(wire::Change {
                furnished_workspace_vector,
                ..
            }) if furnished_workspace_vector.is_empty()
        ));
        assert!(matches!(
            &events[1].response,
            wire::Response::Changed(wire::Change {
                furnished_workspace_vector,
                ..
            }) if furnished_workspace_vector.len() == 1
        ));
        assert_eq!(
            daemon.memory.repositories[0].snapshot.source_revision,
            wire::SourceRevision("two".into())
        );
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
    fn emit(&mut self, query: emission::Query) -> emission::Response;
}

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Configuration {
    pub revision: meta::ConfigurationRevision,
    pub repository_polling: Option<meta::Timeout>,
    pub subscriber_capacity: Option<i64>,
    pub ordinary_peers: Option<wire::PeerList>,
    pub meta_peers: Option<wire::PeerList>,
    pub layout: Option<meta::Layout>,
    pub retention: meta::RetentionPolicy,
}

impl Configuration {
    fn empty() -> Self {
        Self {
            revision: meta::ConfigurationRevision(0),
            repository_polling: None,
            subscriber_capacity: None,
            ordinary_peers: None,
            meta_peers: None,
            layout: None,
            retention: meta::RetentionPolicy::KeepAll,
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
        if self.layout.is_none() {
            missing.push(meta::SettingName::Layout(empty_layout()));
        }
        missing
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
}

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StoredRepository {
    pub owner: wire::Caller,
    pub snapshot: emission::SourceSnapshot,
}

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StoredWorkspace {
    pub owner: wire::Caller,
    pub request: wire::FurnishRequest,
    pub configuration_revision: meta::ConfigurationRevision,
    pub layout: meta::Layout,
    pub retention: meta::RetentionPolicy,
    pub successful: Option<wire::FurnishedWorkspace>,
    pub successful_basis: Vec<wire::RepositoryState>,
    pub pending: Option<emission::Publication>,
}

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MemoryImage {
    pub configuration: Configuration,
    pub repositories: Vec<StoredRepository>,
    pub workspaces: Vec<StoredWorkspace>,
    pub next_event_sequence: i64,
}

impl MemoryImage {
    fn empty() -> Self {
        Self {
            configuration: Configuration::empty(),
            repositories: Vec::new(),
            workspaces: Vec::new(),
            next_event_sequence: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub sequence: i64,
    pub response: wire::Response,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subscription {
    pub id: i64,
    pub initial: Event,
}

#[derive(Clone, Debug)]
struct Subscriber {
    owner: wire::Caller,
    after: i64,
}

pub struct Daemon<E, M> {
    bootstrap: meta::NexusBootstrap,
    effects: E,
    store: M,
    memory: MemoryImage,
    core: BeliefCore,
    events: Vec<Event>,
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
        let memory = store
            .load()
            .map_err(BootstrapError::Memory)?
            .unwrap_or_else(MemoryImage::empty);
        let core = rebuild_core(&memory).map_err(|_| BootstrapError::CorruptState)?;
        Ok(Self {
            bootstrap: bootstrap.nexus,
            effects,
            store,
            memory,
            core,
            events: Vec::new(),
            subscribers: HashMap::new(),
            next_subscriber: 1,
        })
    }

    pub fn handle(&mut self, call: wire::Call) -> wire::Response {
        if !self.authorized_ordinary(&call.caller) {
            return wire::Response::Refused(wire::Refusal::NotAuthorized(call.caller));
        }
        if !self.memory.configuration.ready() {
            return wire::Response::Refused(wire::Refusal::NotReady);
        }
        match call.query {
            wire::Query::Register(repository) => self.register(call.caller, repository),
            wire::Query::Resolve(names) => self
                .core
                .resolve(&names)
                .map(wire::Response::Resolved)
                .unwrap_or_else(|error| wire::Response::Refused(resolve_refusal(error))),
            wire::Query::Furnish(request) => self.furnish(call.caller, request),
            wire::Query::Subscribe => self
                .subscribe(call.caller)
                .map(|subscription| subscription.initial.response)
                .unwrap_or_else(wire::Response::Refused),
        }
    }

    pub fn configure(&mut self, call: meta::MetaCall) -> meta::Response {
        if !self.authorized_meta(&call.caller) {
            return meta::Response::ConfigureRefused(meta::ConfigurationRefusal::NotAuthorized(
                call.caller,
            ));
        }
        let meta::Query::Configure(request) = call.query;
        if request.configuration_revision_option.as_ref()
            != Some(&self.memory.configuration.revision)
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
                    owner: meta::Owner::Nexus,
                    setting_name: setting_name(&set),
                    string: message.into(),
                },
            ));
        }
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
            meta::Set::Layout(value) => self.memory.configuration.layout = Some(value.clone()),
            meta::Set::GenerationRetention(value) => {
                self.memory.configuration.retention = value.clone()
            }
            _ => {
                return meta::Response::ConfigureRefused(meta::ConfigurationRefusal::Invalid(
                    meta::Invalid {
                        owner: meta::Owner::Translator,
                        setting_name: setting_name(&set),
                        string: "translator-owned setting".into(),
                    },
                ));
            }
        }
        self.memory.configuration.revision.0 += 1;
        if self.persist().is_err() {
            return meta::Response::ConfigureRefused(meta::ConfigurationRefusal::Unavailable);
        }
        meta::Response::Configured(meta::ConfiguredState {
            set,
            owner: meta::Owner::Nexus,
            configuration_revision: self.memory.configuration.revision.clone(),
            readiness: self.memory.configuration.readiness(),
        })
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
            match self
                .effects
                .emit(emission::Query::Snapshot(repository.clone()))
            {
                emission::Response::Snapshotted(snapshot)
                    if snapshot.source_revision == old_revision => {}
                emission::Response::Snapshotted(snapshot) => self.apply_snapshot(snapshot),
                emission::Response::EmissionRefused(emission::EmissionRefusal::Source(source)) => {
                    let failure = map_source(source, wire::SourceOperation::Watching);
                    self.event(wire::Response::WatchFailed(wire::WatchFailure {
                        repository,
                        source_revision_option: Some(old_revision),
                        source_failure: failure,
                    }));
                }
                _ => self.event(wire::Response::WatchFailed(watch_unavailable(
                    repository,
                    Some(old_revision),
                ))),
            }
        }
        self.events[offset..].to_vec()
    }

    pub fn drain(&mut self, id: i64) -> Option<Vec<Event>> {
        let subscriber = self.subscribers.get_mut(&id)?;
        let events: Vec<_> = self
            .events
            .iter()
            .filter(|event| event.sequence > subscriber.after)
            .cloned()
            .collect();
        if let Some(last) = events.last() {
            subscriber.after = last.sequence;
        }
        Some(events)
    }

    pub fn unsubscribe(&mut self, id: i64) {
        self.subscribers.remove(&id);
    }

    pub fn subscriber_owner(&self, id: i64) -> Option<&wire::Caller> {
        Some(&self.subscribers.get(&id)?.owner)
    }

    fn register(&mut self, owner: wire::Caller, repository: wire::Repository) -> wire::Response {
        if self
            .repository(&repository)
            .is_some_and(|stored| !same_owner(&stored.owner, &owner))
        {
            return wire::Response::Refused(wire::Refusal::NotAuthorized(owner));
        }
        let first = match self
            .effects
            .emit(emission::Query::Snapshot(repository.clone()))
        {
            emission::Response::Snapshotted(snapshot) => snapshot,
            emission::Response::EmissionRefused(refusal) => {
                return wire::Response::Refused(map_emission(refusal));
            }
            _ => return wire::Response::Refused(wire::Refusal::NotReady),
        };
        if self.install_repository(owner, first.clone()).is_err() {
            return wire::Response::Refused(memory_refusal(wire::MemoryOperation::Recording));
        }
        // Immediate next observation closes the register/watch handoff window.
        if let emission::Response::Snapshotted(next) = self
            .effects
            .emit(emission::Query::Snapshot(repository.clone()))
        {
            if next.source_revision != first.source_revision {
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
        owner: wire::Caller,
        snapshot: emission::SourceSnapshot,
    ) -> Result<(), ()> {
        self.core.observe(snapshot.clone().into()).map_err(|_| ())?;
        if let Some(stored) = self.repository_mut(&snapshot.repository) {
            stored.snapshot = snapshot;
        } else {
            self.memory
                .repositories
                .push(StoredRepository { owner, snapshot });
        }
        self.sort();
        self.persist().map_err(|_| ())
    }

    fn furnish(&mut self, owner: wire::Caller, request: wire::FurnishRequest) -> wire::Response {
        if self
            .workspace(&request.workspace)
            .is_some_and(|stored| !same_owner(&stored.owner, &owner))
        {
            return wire::Response::Refused(wire::Refusal::NotAuthorized(owner));
        }
        let revision = self.memory.configuration.revision.clone();
        let layout = self.memory.configuration.layout.clone().expect("ready");
        let retention = self.memory.configuration.retention.clone();
        let plan = match self.core.prepare_furnish(request.clone()) {
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
            stored.retention = retention;
            stored.pending = Some(publication.clone());
        } else {
            self.memory.workspaces.push(StoredWorkspace {
                owner,
                request: request.clone(),
                configuration_revision: revision,
                layout,
                retention,
                successful: None,
                successful_basis: Vec::new(),
                pending: Some(publication.clone()),
            });
        }
        self.sort();
        if self.persist().is_err() {
            return wire::Response::Refused(memory_refusal(
                wire::MemoryOperation::BeginningPublication,
            ));
        }
        self.publish(publication)
    }

    fn publish(&mut self, publication: emission::Publication) -> wire::Response {
        match self.effects.emit(emission::Query::Publish(publication)) {
            emission::Response::Published(state) => {
                let furnished = match self.core.complete_publication(&state) {
                    Ok(furnished) => furnished,
                    Err(_) => return wire::Response::Refused(file_refusal(state.workspace)),
                };
                let before = self.workspace(&state.workspace).cloned();
                if let Some(stored) = self.workspace_mut(&state.workspace) {
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
                        self.restore_pending_core(before.clone());
                        if let Some(stored) = self.workspace_mut(&state.workspace) {
                            *stored = before;
                        }
                    }
                    let answer =
                        wire::Response::MemoryAfterPublication(wire::MemoryAfterPublication {
                            workspace: state.workspace,
                            generation_id: state.generation_id,
                            publication_revision: state.publication_revision,
                            memory_failure: memory_failure(
                                wire::MemoryOperation::CompletingPublication,
                                error,
                            ),
                        });
                    self.event(answer.clone());
                    return answer;
                }
                self.clean_after_success(&furnished.workspace);
                wire::Response::Furnished(furnished)
            }
            emission::Response::EmissionRefused(refusal) => {
                wire::Response::Refused(map_emission(refusal))
            }
            _ => wire::Response::Refused(wire::Refusal::NotReady),
        }
    }

    fn clean_after_success(&mut self, workspace: &wire::Workspace) {
        let Some(stored) = self.workspace(workspace).cloned() else {
            return;
        };
        if matches!(stored.retention, meta::RetentionPolicy::KeepAll) {
            return;
        }
        let protected = self
            .memory
            .workspaces
            .iter()
            .filter_map(|record| record.pending.as_ref())
            .map(|pending| pending.generation_id.clone())
            .collect();
        let request = emission::CleanupRequest {
            workspace: workspace.clone(),
            layout: stored.layout,
            retention_policy: stored.retention,
            protected_generations: emission::ProtectedGenerations(protected),
        };
        if let emission::Response::CleanupRefused(refusal) =
            self.effects.emit(emission::Query::Clean(request))
        {
            // Cleanup is separate evidence. The successful publication remains true.
            self.event(wire::Response::RefurnishFailed(wire::RefurnishFailure {
                workspace: workspace.clone(),
                repository: wire::Repository(wire::RepositoryPath(String::new())),
                source_revision: wire::SourceRevision(String::new()),
                refusal: cleanup_refusal(refusal),
            }));
        }
    }

    fn recover_one(&mut self, workspace: &wire::Workspace) -> Option<wire::Response> {
        let stored = self.workspace(workspace)?.clone();
        let response = self.effects.emit(emission::Query::InspectPublication(
            emission::InspectionRequest {
                workspace: workspace.clone(),
                layout: stored.layout.clone(),
            },
        ));
        match response {
            emission::Response::Inspected(inspection) => {
                match self.core.recover_publication(&inspection) {
                    Ok(RecoveryOutcome::Finalized(furnished)) => {
                        if let Some(record) = self.workspace_mut(workspace) {
                            record.successful = Some(furnished.clone());
                            record.successful_basis = record
                                .pending
                                .as_ref()
                                .map(|pending| pending.repository_states.0.clone())
                                .unwrap_or_default();
                            record.pending = None;
                        }
                        if self.persist().is_err() {
                            return Some(wire::Response::MemoryAfterPublication(
                                wire::MemoryAfterPublication {
                                    workspace: workspace.clone(),
                                    generation_id: furnished.generation_id,
                                    publication_revision: furnished.publication_revision,
                                    memory_failure: memory_failure(
                                        wire::MemoryOperation::CompletingPublication,
                                        MemoryError::Unavailable,
                                    ),
                                },
                            ));
                        }
                        self.clean_after_success(workspace);
                        Some(wire::Response::Furnished(furnished))
                    }
                    Ok(RecoveryOutcome::Retry) => Some(self.furnish(stored.owner, stored.request)),
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
        let repository = snapshot.repository.clone();
        let revision = snapshot.source_revision.clone();
        let change = match self.core.observe(snapshot.clone().into()) {
            Ok(change) => change,
            Err(_) => {
                self.event(wire::Response::WatchFailed(watch_unavailable(
                    repository,
                    Some(revision),
                )));
                return;
            }
        };
        if let Some(stored) = self.repository_mut(&repository) {
            stored.snapshot = snapshot;
        }
        if self.persist().is_err() {
            return;
        }
        self.event(wire::Response::Changed(wire::Change {
            repository: repository.clone(),
            source_revision: revision.clone(),
            module_name_vector: change.changed,
            furnished_workspace_vector: Vec::new(),
        }));
        for workspace in change.retry_workspaces {
            let Some(stored) = self.workspace(&workspace).cloned() else {
                continue;
            };
            match self.furnish(stored.owner, stored.request) {
                wire::Response::Furnished(furnished) => {
                    self.event(wire::Response::Changed(wire::Change {
                        repository: repository.clone(),
                        source_revision: revision.clone(),
                        module_name_vector: Vec::new(),
                        furnished_workspace_vector: vec![furnished],
                    }));
                }
                wire::Response::Refused(refusal) => {
                    self.event(wire::Response::RefurnishFailed(wire::RefurnishFailure {
                        workspace,
                        repository: repository.clone(),
                        source_revision: revision.clone(),
                        refusal,
                    }));
                }
                response => self.event(response),
            }
        }
    }

    fn subscribe(&mut self, owner: wire::Caller) -> Result<Subscription, wire::Refusal> {
        let capacity = self.memory.configuration.subscriber_capacity.unwrap_or(0);
        if self.subscribers.len() as i64 >= capacity {
            return Err(wire::Refusal::SubscriberCapacityReached(capacity));
        }
        let id = self.next_subscriber;
        self.next_subscriber += 1;
        let initial = self.next_event(wire::Response::Subscribed(self.state()));
        self.subscribers.insert(
            id,
            Subscriber {
                owner,
                after: initial.sequence,
            },
        );
        Ok(Subscription { id, initial })
    }

    fn state(&self) -> wire::BeliefState {
        let mut furnished: Vec<_> = self
            .memory
            .workspaces
            .iter()
            .filter_map(|workspace| workspace.successful.clone())
            .collect();
        furnished.sort_by_key(|value| value.workspace.0 .0.clone());
        wire::BeliefState {
            repository_state_vector: self.core.repository_states(),
            furnished_workspace_vector: furnished,
        }
    }

    fn restore_pending_core(&mut self, stored: StoredWorkspace) {
        if let Ok(plan) = self.core.prepare_furnish(stored.request) {
            let _ = self.core.begin_publication(
                &plan,
                PublicationContext {
                    owner: stored.owner,
                    configuration_revision: stored.configuration_revision,
                    layout: stored.layout,
                    retention_policy: stored.retention,
                },
            );
        }
    }

    fn event(&mut self, response: wire::Response) {
        let event = self.next_event(response);
        self.events.push(event);
        let _ = self.persist();
    }

    fn next_event(&mut self, response: wire::Response) -> Event {
        let sequence = self.memory.next_event_sequence;
        self.memory.next_event_sequence += 1;
        Event { sequence, response }
    }

    fn persist(&mut self) -> Result<(), MemoryError> {
        self.store.store(&self.memory)
    }

    fn authorized_ordinary(&self, caller: &wire::Caller) -> bool {
        self.memory
            .configuration
            .ordinary_peers
            .as_ref()
            .is_some_and(|peers| peer_allowed(peers, caller))
    }

    fn authorized_meta(&self, caller: &wire::Caller) -> bool {
        paired(&self.bootstrap.paired_process, caller)
            || self
                .memory
                .configuration
                .meta_peers
                .as_ref()
                .is_some_and(|peers| peer_allowed(peers, caller))
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
