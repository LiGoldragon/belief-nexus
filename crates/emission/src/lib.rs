#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    clippy::type_complexity
)]
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct EmissionAttempt {
    pub configuration_revision: meta_signal_belief_nexus::ConfigurationRevision,
    pub translator_configuration: meta_signal_belief_nexus::TranslatorConfiguration,
    pub query: Query,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct EmissionAttemptOutcome {
    pub configuration_revision: meta_signal_belief_nexus::ConfigurationRevision,
    pub response: Response,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Content(pub String);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct AbsolutePath(pub String);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ModuleSource {
    pub module_name: signal_belief_nexus::ModuleName,
    pub relative_path: signal_belief_nexus::RelativePath,
    pub placement_option: std::option::Option<signal_belief_nexus::Placement>,
    pub content: Content,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct SourceSnapshot {
    pub repository: signal_belief_nexus::Repository,
    pub source_revision: signal_belief_nexus::SourceRevision,
    pub module_source_vector: std::vec::Vec<ModuleSource>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct SkillFile {
    pub relative_path: signal_belief_nexus::RelativePath,
    pub content: Content,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct WorkspaceImage {
    pub harness: signal_belief_nexus::Harness,
    pub layout: meta_signal_belief_nexus::Layout,
    pub content: Content,
    pub skill_file_vector: std::vec::Vec<SkillFile>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct RepositoryStates(pub std::vec::Vec<signal_belief_nexus::RepositoryState>);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct FoundModules(pub std::vec::Vec<signal_belief_nexus::ModuleName>);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct MissingModules(pub std::vec::Vec<signal_belief_nexus::ModuleName>);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct PriorPublication(
    pub std::option::Option<signal_belief_nexus::PublicationRevision>,
);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct IntendedPublication(pub signal_belief_nexus::PublicationRevision);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct PublicationManifest {
    pub generation_id: signal_belief_nexus::GenerationId,
    pub intended_publication: IntendedPublication,
    pub repository_states: RepositoryStates,
    pub found_modules: FoundModules,
    pub missing_modules: MissingModules,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Publication {
    pub workspace: signal_belief_nexus::Workspace,
    pub generation_id: signal_belief_nexus::GenerationId,
    pub prior_publication: PriorPublication,
    pub intended_publication: IntendedPublication,
    pub workspace_image: WorkspaceImage,
    pub repository_states: RepositoryStates,
    pub found_modules: FoundModules,
    pub missing_modules: MissingModules,
    pub retention_policy: meta_signal_belief_nexus::RetentionPolicy,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct PublicationIdentity {
    pub generation_id: signal_belief_nexus::GenerationId,
    pub publication_revision: signal_belief_nexus::PublicationRevision,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct PublicationState {
    pub workspace: signal_belief_nexus::Workspace,
    pub publication_identity: PublicationIdentity,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct InspectionRequest {
    pub workspace: signal_belief_nexus::Workspace,
    pub publication_identity: PublicationIdentity,
    pub layout: meta_signal_belief_nexus::Layout,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct PublicationInspection {
    pub workspace: signal_belief_nexus::Workspace,
    pub publication_identity_option: std::option::Option<PublicationIdentity>,
    pub publication_manifest_option: std::option::Option<PublicationManifest>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct CleanupRequest {
    pub workspace: signal_belief_nexus::Workspace,
    pub publication_identity: PublicationIdentity,
    pub layout: meta_signal_belief_nexus::Layout,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum CleanupOutcome {
    Removed,
    AlreadyAbsent,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct CleanupState {
    pub workspace: signal_belief_nexus::Workspace,
    pub publication_identity: PublicationIdentity,
    pub cleanup_outcome: CleanupOutcome,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum SourceOperation {
    ReadingMain,
    ReadingTree,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum PublicationOperation {
    StagingSettings,
    StagingGeneration,
    WritingManifest,
    ReplacingPointer,
    InspectingPointer,
    InspectingManifest,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum InvalidPlacement {
    Unknown(String),
    NonScalar,
    Duplicate,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct InvalidModule {
    pub module_name: signal_belief_nexus::ModuleName,
    pub relative_path: signal_belief_nexus::RelativePath,
    pub invalid_placement: InvalidPlacement,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum SourceEvidence {
    NotRepository,
    MainUnavailable,
    RevisionUnavailable,
    InvalidModule(InvalidModule),
    Translator(signal_belief_nexus::Diagnostic),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct SourceRefusal {
    pub repository: signal_belief_nexus::Repository,
    pub source_operation: SourceOperation,
    pub source_evidence: SourceEvidence,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct BoundaryRoot(pub AbsolutePath);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct BoundaryCandidate(pub AbsolutePath);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct BoundaryEvidence {
    pub boundary_root: BoundaryRoot,
    pub boundary_candidate: BoundaryCandidate,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ExpectedRevision(
    pub std::option::Option<signal_belief_nexus::PublicationRevision>,
);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ObservedRevision(
    pub std::option::Option<signal_belief_nexus::PublicationRevision>,
);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct RevisionEvidence {
    pub expected_revision: ExpectedRevision,
    pub observed_revision: ObservedRevision,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum PublicationEvidence {
    OutsideBoundary(BoundaryEvidence),
    DestinationUnavailable(AbsolutePath),
    PriorRevisionMismatch(RevisionEvidence),
    OwnedPointerMismatch,
    ManifestMismatch,
    SettingsConflict,
    InvalidSettings,
    PermissionDenied,
    Translator(signal_belief_nexus::Diagnostic),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct PublicationRefusal {
    pub workspace: signal_belief_nexus::Workspace,
    pub publication_operation: PublicationOperation,
    pub publication_evidence: PublicationEvidence,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum EmissionRefusal {
    Source(SourceRefusal),
    Publication(PublicationRefusal),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Query {
    Snapshot(signal_belief_nexus::Repository),
    Publish(Publication),
    InspectPublication(InspectionRequest),
    Clean(CleanupRequest),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Response {
    Snapshotted(SourceSnapshot),
    Published(PublicationState),
    Inspected(PublicationInspection),
    Cleaned(CleanupState),
    CleanupRefused(signal_belief_nexus::FileFailure),
    EmissionRefused(EmissionRefusal),
}
pub const ETHOS: &str = include_str!("../../../emission.ethos");

impl signal::Contracted for Query {
    const CONTRACT_SOURCE: &'static str = ETHOS;
}

use signal::{ByteViewable as _, Restorable as _, Signal, Signalizable as _};
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::symlink;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// The process-side implementation of Belief's external effects.
///
/// The executable is supplied configuration. Each publication carries the
/// coherent retention policy selected by the Nexus. The Nexus sends only
/// typed `Query` values and never observes command lines or paths
/// that have not passed through this boundary.
#[derive(Clone, Debug)]
pub struct FileEmission {
    git_executable: PathBuf,
}

impl FileEmission {
    pub fn new(git_executable: PathBuf) -> Result<Self, String> {
        if git_executable.as_os_str().is_empty() { return Err("git executable must be named".into()); }
        Ok(Self { git_executable })
    }

    pub fn execute(&self, query: Query) -> Response {
        match query {
            Query::Snapshot(repository) => self.snapshot(repository),
            Query::Publish(publication) => self.publish(publication),
            Query::InspectPublication(request) => self.inspect(request),
            Query::Clean(request) => self.clean(request),
        }
    }

    fn snapshot(&self, repository: signal_belief_nexus::Repository) -> Response {
        match self.snapshot_result(repository.clone()) {
            Ok(snapshot) => Response::Snapshotted(snapshot),
            Err(refusal) => Response::EmissionRefused(EmissionRefusal::Source(refusal)),
        }
    }

    fn snapshot_result(
        &self,
        repository: signal_belief_nexus::Repository,
    ) -> Result<SourceSnapshot, SourceRefusal> {
        let root = PathBuf::from(&(repository.0).0);
        if !root.is_dir() {
            return Err(SourceRefusal {
                repository,
                source_operation: SourceOperation::ReadingMain,
                source_evidence: SourceEvidence::NotRepository,
            });
        }
        if self.git(&root, ["rev-parse", "--git-dir"]).is_err() {
            return Err(SourceRefusal {
                repository,
                source_operation: SourceOperation::ReadingMain,
                source_evidence: SourceEvidence::NotRepository,
            });
        }

        // Resolve the commit once. Every later read names this object, so a
        // concurrently moving main cannot braid two revisions into a snapshot.
        let revision = self.git(&root, ["rev-parse", "--verify", "refs/heads/main^{commit}"])
            .map_err(|diagnostic| SourceRefusal {
                repository: repository.clone(),
                source_operation: SourceOperation::ReadingMain,
                source_evidence: SourceEvidence::Translator(diagnostic),
            })?;
        let revision = String::from_utf8(revision)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| is_git_object_id(value))
            .ok_or_else(|| SourceRefusal {
                repository: repository.clone(),
                source_operation: SourceOperation::ReadingMain,
                source_evidence: SourceEvidence::RevisionUnavailable,
            })?;

        let listing = self
            .git_bytes(&root, [OsStr::new("ls-tree"), OsStr::new("-r"), OsStr::new("-z"), OsStr::new("--name-only"), OsStr::new(&revision)])
            .map_err(|diagnostic| SourceRefusal {
                repository: repository.clone(),
                source_operation: SourceOperation::ReadingTree,
                source_evidence: SourceEvidence::Translator(diagnostic),
            })?;
        let mut modules = Vec::new();
        for raw in listing.split(|byte| *byte == 0).filter(|path| !path.is_empty()) {
            let path = std::str::from_utf8(raw).map_err(|_| SourceRefusal {
                repository: repository.clone(),
                source_operation: SourceOperation::ReadingTree,
                source_evidence: SourceEvidence::RevisionUnavailable,
            })?;
            let Some(module_name) = module_name_from_path(path) else { continue };
            let object = format!("{revision}:{path}");
            let content = self
                .git_bytes(&root, [OsStr::new("show"), OsStr::new(&object)])
                .map_err(|diagnostic| SourceRefusal {
                    repository: repository.clone(),
                    source_operation: SourceOperation::ReadingTree,
                    source_evidence: SourceEvidence::Translator(diagnostic),
                })?;
            let content = String::from_utf8(content).map_err(|_| SourceRefusal {
                repository: repository.clone(),
                source_operation: SourceOperation::ReadingTree,
                source_evidence: SourceEvidence::RevisionUnavailable,
            })?;
            let relative_path = signal_belief_nexus::RelativePath(path.into());
            let placement_option = placement_from_frontmatter(&content, &module_name, &relative_path)
                .map_err(|invalid| SourceRefusal {
                    repository: repository.clone(),
                    source_operation: SourceOperation::ReadingTree,
                    source_evidence: SourceEvidence::InvalidModule(invalid),
                })?;
            modules.push(ModuleSource {
                module_name,
                relative_path,
                placement_option,
                content: Content(content),
            });
        }
        modules.sort_by(|left, right| (left.relative_path.0).cmp(&right.relative_path.0));
        Ok(SourceSnapshot {
            repository,
            source_revision: signal_belief_nexus::SourceRevision(revision),
            module_source_vector: modules,
        })
    }

    fn git<const N: usize>(&self, root: &Path, args: [&str; N]) -> Result<Vec<u8>, signal_belief_nexus::Diagnostic> {
        self.git_bytes(root, args.map(OsStr::new))
    }

    fn git_bytes<I, S>(&self, root: &Path, args: I) -> Result<Vec<u8>, signal_belief_nexus::Diagnostic>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new(&self.git_executable)
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .map_err(|error| self.diagnostic(None, error.to_string()))?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(self.diagnostic(output.status.code().map(|value| value as i64), String::from_utf8_lossy(&output.stderr).into_owned()))
        }
    }

    fn diagnostic(&self, status: Option<i64>, text: String) -> signal_belief_nexus::Diagnostic {
        signal_belief_nexus::Diagnostic {
            executable_path: signal_belief_nexus::ExecutablePath(self.git_executable.to_string_lossy().into_owned()),
            exit_status_option: status.map(signal_belief_nexus::ExitStatus),
            string: text,
        }
    }

    fn publish(&self, publication: Publication) -> Response {
        match self.publish_result(&publication) {
            Ok(state) => Response::Published(state),
            Err(refusal) => Response::EmissionRefused(EmissionRefusal::Publication(refusal)),
        }
    }

    fn publish_result(&self, publication: &Publication) -> Result<PublicationState, PublicationRefusal> {
        let workspace = PathBuf::from(&((publication.workspace.0).0));
        let operation = PublicationOperation::StagingGeneration;
        let paths = PublicationPaths::new(&workspace, &publication.workspace_image.layout)
            .map_err(|evidence| publication_refusal(publication, operation.clone(), evidence))?;
        paths.prevalidate(publication)
            .map_err(|evidence| publication_refusal(publication, operation.clone(), evidence))?;
        let identity = publication.identity();
        let existing = paths.resolve_publication(&identity)
            .map_err(|evidence| publication_refusal(publication, operation.clone(), evidence))?;
        let intended_manifest = publication.manifest();
        if existing.as_ref().is_some_and(|value| value.manifest != intended_manifest) {
            return Err(publication_refusal(publication, PublicationOperation::InspectingManifest, PublicationEvidence::ManifestMismatch));
        }

        fs::create_dir_all(&paths.generations)
            .and_then(|_| fs::create_dir_all(paths.settings.parent().expect("settings has parent")))
            .and_then(|_| fs::create_dir_all(paths.skills_link.parent().expect("skills has parent")))
            .map_err(|error| publication_refusal(publication, operation.clone(), io_evidence(&paths.workspace, error)))?;

        let current = paths.inspect_current(publication)
            .map_err(|evidence| publication_refusal(publication, PublicationOperation::InspectingPointer, evidence))?;
        let observed = current.as_ref().map(|(_, manifest)| (manifest.intended_publication.0).clone());
        if observed != publication.prior_publication.0 {
            return Err(publication_refusal(
                publication,
                PublicationOperation::InspectingManifest,
                PublicationEvidence::PriorRevisionMismatch(RevisionEvidence {
                    expected_revision: ExpectedRevision(publication.prior_publication.0.clone()),
                    observed_revision: ObservedRevision(observed),
                }),
            ));
        }

        paths.prepare_capability_links(publication)
            .map_err(|evidence| publication_refusal(publication, PublicationOperation::StagingGeneration, evidence))?;
        paths.write_settings(publication)
            .map_err(|evidence| publication_refusal(publication, PublicationOperation::StagingSettings, evidence))?;

        let generation = if let Some(existing) = existing {
            existing.path
        } else {
            let generation = paths.publication_directory(&identity)
                .map_err(|evidence| publication_refusal(publication, operation.clone(), evidence))?;
            paths.stage_generation(publication, &generation, &intended_manifest)
                .map_err(|evidence| publication_refusal(publication, operation.clone(), evidence))?;
            generation
        };

        paths.replace_current(publication, &generation)
            .map_err(|evidence| publication_refusal(publication, PublicationOperation::ReplacingPointer, evidence))?;
        Ok(PublicationState {
            workspace: publication.workspace.clone(),
            publication_identity: identity,
        })
    }

    fn inspect(&self, request: InspectionRequest) -> Response {
        let workspace = PathBuf::from(&((request.workspace.0).0));
        let paths = match PublicationPaths::new(&workspace, &request.layout) {
            Ok(paths) => paths,
            Err(evidence) => return Response::EmissionRefused(EmissionRefusal::Publication(PublicationRefusal {
                workspace: request.workspace,
                publication_operation: PublicationOperation::InspectingPointer,
                publication_evidence: evidence,
            })),
        };
        if let Err(evidence) = paths.resolve_publication(&request.publication_identity) {
            return Response::EmissionRefused(EmissionRefusal::Publication(PublicationRefusal {
                workspace: request.workspace,
                publication_operation: PublicationOperation::InspectingManifest,
                publication_evidence: evidence,
            }));
        }
        match paths.inspect_untrusted() {
            Ok((pointer, manifest)) => Response::Inspected(PublicationInspection {
                workspace: request.workspace,
                publication_identity_option: pointer,
                publication_manifest_option: manifest,
            }),
            Err(evidence) => Response::EmissionRefused(EmissionRefusal::Publication(PublicationRefusal {
                workspace: request.workspace,
                publication_operation: PublicationOperation::InspectingManifest,
                publication_evidence: evidence,
            })),
        }
    }

    fn clean(&self, request: CleanupRequest) -> Response {
        let workspace = PathBuf::from(&((request.workspace.0).0));
        let paths = match PublicationPaths::new(&workspace, &request.layout) {
            Ok(paths) => paths,
            Err(evidence) => return Response::CleanupRefused(cleanup_failure(&request.workspace, evidence)),
        };
        match paths.cleanup(&request.publication_identity) {
            Ok(cleanup_outcome) => Response::Cleaned(CleanupState {
                workspace: request.workspace, publication_identity: request.publication_identity, cleanup_outcome,
            }),
            Err(evidence) => Response::CleanupRefused(cleanup_failure(&request.workspace, evidence)),
        }
    }
}

fn placement_from_frontmatter(
    content: &str,
    module_name: &signal_belief_nexus::ModuleName,
    relative_path: &signal_belief_nexus::RelativePath,
) -> Result<Option<signal_belief_nexus::Placement>, InvalidModule> {
    let mut lines = content.lines();
    if lines.next() != Some("---") { return Ok(None); }
    let frontmatter = lines.take_while(|line| *line != "---");
    let mut found = None;
    for line in frontmatter {
        let Some((key, raw)) = line.split_once(':') else { continue };
        if key.trim() != "placement" { continue; }
        if found.is_some() {
            return Err(InvalidModule { module_name: module_name.clone(), relative_path: relative_path.clone(), invalid_placement: InvalidPlacement::Duplicate });
        }
        let value = raw.trim();
        let placement = match value {
            "system-prompt" => signal_belief_nexus::Placement::SystemPrompt,
            "skills" => signal_belief_nexus::Placement::Skills,
            "" => return Err(InvalidModule { module_name: module_name.clone(), relative_path: relative_path.clone(), invalid_placement: InvalidPlacement::NonScalar }),
            value if matches!(value.as_bytes().first(), Some(b'[' | b'{' | b'|' | b'>')) => return Err(InvalidModule { module_name: module_name.clone(), relative_path: relative_path.clone(), invalid_placement: InvalidPlacement::NonScalar }),
            value => return Err(InvalidModule { module_name: module_name.clone(), relative_path: relative_path.clone(), invalid_placement: InvalidPlacement::Unknown(value.into()) }),
        };
        found = Some(placement);
    }
    Ok(found)
}

impl Publication {
    fn identity(&self) -> PublicationIdentity {
        PublicationIdentity {
            generation_id: self.generation_id.clone(),
            publication_revision: self.intended_publication.0.clone(),
        }
    }

    fn manifest(&self) -> PublicationManifest {
        PublicationManifest {
            generation_id: self.generation_id.clone(),
            intended_publication: self.intended_publication.clone(),
            repository_states: self.repository_states.clone(),
            found_modules: self.found_modules.clone(),
            missing_modules: self.missing_modules.clone(),
        }
    }
}

impl PublicationManifest {
    fn identity(&self) -> PublicationIdentity {
        PublicationIdentity {
            generation_id: self.generation_id.clone(),
            publication_revision: self.intended_publication.0.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LegacyContentDirectory(signal_belief_nexus::GenerationId);

#[derive(Clone, Debug, PartialEq, Eq)]
struct PublicationDirectory(PublicationIdentity);

#[derive(Clone, Debug, PartialEq, Eq)]
enum PublicationAddress {
    Legacy(LegacyContentDirectory),
    Publication(PublicationDirectory),
}

#[derive(Debug)]
struct ResolvedPublication {
    path: PathBuf,
    manifest: PublicationManifest,
}

#[derive(Debug)]
struct PublicationPaths {
    workspace: PathBuf,
    owned: PathBuf,
    generations: PathBuf,
    current: PathBuf,
    settings: PathBuf,
    instructions: PathBuf,
    skills_link: PathBuf,
}

impl PublicationPaths {
    fn new(workspace: &Path, layout: &meta_signal_belief_nexus::Layout) -> Result<Self, PublicationEvidence> {
        if !workspace.is_dir()
            || fs::symlink_metadata(workspace).map(|metadata| metadata.file_type().is_symlink()).unwrap_or(true)
        {
            return Err(PublicationEvidence::DestinationUnavailable(AbsolutePath(workspace.to_string_lossy().into_owned())));
        }
        let owned_rel = safe_relative(&((layout.owned_root.0).0))?;
        let generations_rel = safe_relative(&((layout.generations_directory.0).0))?;
        let current_rel = safe_relative(&((layout.current_link.0).0))?;
        let settings_rel = safe_relative(&((layout.settings_file.0).0))?;
        let instructions_rel = safe_relative(&((layout.instructions_file.0).0))?;
        let skills_rel = safe_relative(&((layout.skills_directory.0).0))?;
        let owned = workspace.join(owned_rel);
        Ok(Self {
            workspace: workspace.to_owned(),
            generations: owned.join(generations_rel),
            current: owned.join(current_rel),
            settings: workspace.join(settings_rel),
            instructions: instructions_rel,
            skills_link: workspace.join(skills_rel),
            owned,
        })
    }

    fn prevalidate(&self, publication: &Publication) -> Result<(), PublicationEvidence> {
        for path in [&self.owned, &self.generations, &self.settings, &self.skills_link] {
            ensure_no_ancestor_symlink(&self.workspace, path)?;
        }
        if let Ok(metadata) = fs::symlink_metadata(&self.owned) {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(boundary(&self.workspace, &self.owned));
            }
        }
        match publication.workspace_image.harness {
            signal_belief_nexus::Harness::Codex => if let Ok(metadata) = fs::symlink_metadata(&self.settings) {
                if !metadata.is_file() || metadata.file_type().is_symlink() { return Err(boundary(&self.workspace, &self.settings)); }
            },
            signal_belief_nexus::Harness::Claude => if let Ok(metadata) = fs::symlink_metadata(&self.settings) {
                if !metadata.file_type().is_symlink() || fs::read_link(&self.settings).ok().as_deref() != Some(self.current.join(&self.instructions).as_path()) { return Err(boundary(&self.workspace, &self.settings)); }
            },
        }
        if let Ok(metadata) = fs::symlink_metadata(&self.skills_link) {
            let expected = match publication.workspace_image.harness { signal_belief_nexus::Harness::Codex => PathBuf::from("../.belief/current/skills"), signal_belief_nexus::Harness::Claude => self.current.join("skills") };
            if !metadata.file_type().is_symlink() || fs::read_link(&self.skills_link).ok().as_deref() != Some(expected.as_path()) {
                return Err(boundary(&self.workspace, &self.skills_link));
            }
        }
        let _ = self.rendered_settings(publication)?;
        let publication_directory = self.publication_directory(&publication.identity())?;
        let mut skill_paths = Vec::new();
        for skill in &publication.workspace_image.skill_file_vector {
            let relative = safe_relative(&skill.relative_path.0)?;
            if skill_paths.iter().any(|existing: &PathBuf|
                relative.starts_with(existing) || existing.starts_with(&relative))
            {
                return Err(PublicationEvidence::PermissionDenied);
            }
            skill_paths.push(relative.clone());
            let candidate = publication_directory.join("skills").join(relative);
            if !candidate.starts_with(&self.generations) {
                return Err(boundary(&self.generations, &candidate));
            }
        }
        let _ = self.legacy_directory(&publication.generation_id)?;
        Ok(())
    }

    fn validate_generation(&self, generation: &signal_belief_nexus::GenerationId) -> Result<(), PublicationEvidence> {
        if generation.0.is_empty() || generation.0.contains('/') || generation.0.contains('\\') || generation.0 == "." || generation.0 == ".." {
            return Err(boundary(&self.generations, &self.generations.join(&generation.0)));
        }
        Ok(())
    }

    fn legacy_directory(&self, generation: &signal_belief_nexus::GenerationId) -> Result<PathBuf, PublicationEvidence> {
        self.validate_generation(generation)?;
        Ok(self.generations.join(&generation.0))
    }

    fn publication_component(&self, identity: &PublicationIdentity) -> Result<String, PublicationEvidence> {
        self.validate_generation(&identity.generation_id)?;
        if identity.publication_revision.0 <= 0 {
            return Err(boundary(&self.generations, &self.generations.join(identity.publication_revision.0.to_string())));
        }
        let mut hex = String::with_capacity(identity.generation_id.0.len() * 2);
        for byte in identity.generation_id.0.as_bytes() {
            use std::fmt::Write as _;
            write!(&mut hex, "{byte:02x}").expect("writing to String cannot fail");
        }
        Ok(format!("publication-{}-{hex}", identity.publication_revision.0))
    }

    fn publication_directory(&self, identity: &PublicationIdentity) -> Result<PathBuf, PublicationEvidence> {
        Ok(self.generations.join(self.publication_component(identity)?))
    }

    fn inspect_candidate(&self, path: &Path) -> Result<Option<(PublicationAddress, PublicationManifest)>, PublicationEvidence> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(io_evidence(path, error)),
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(PublicationEvidence::ManifestMismatch);
        }
        let manifest = read_manifest(&path.join("manifest.signal")).map_err(|_| PublicationEvidence::ManifestMismatch)?;
        let component = path.file_name().and_then(|value| value.to_str()).ok_or(PublicationEvidence::ManifestMismatch)?;
        let legacy = component == manifest.generation_id.0;
        let publication = component == self.publication_component(&manifest.identity())?;
        let address = match (legacy, publication) {
            (true, false) => PublicationAddress::Legacy(LegacyContentDirectory(manifest.generation_id.clone())),
            (false, true) => PublicationAddress::Publication(PublicationDirectory(manifest.identity())),
            _ => return Err(PublicationEvidence::ManifestMismatch),
        };
        Ok(Some((address, manifest)))
    }

    fn resolve_publication(&self, identity: &PublicationIdentity) -> Result<Option<ResolvedPublication>, PublicationEvidence> {
        let legacy = self.legacy_directory(&identity.generation_id)?;
        let publication = self.publication_directory(identity)?;
        if legacy == publication {
            return Err(PublicationEvidence::ManifestMismatch);
        }
        let legacy_value = self.inspect_candidate(&legacy);
        let publication_value = self.inspect_candidate(&publication);
        let legacy_value = legacy_value?;
        let publication_value = publication_value?;
        let mut matching = Vec::new();
        if let Some((_, manifest)) = legacy_value {
            if manifest.identity() == *identity {
                matching.push(ResolvedPublication { path: legacy, manifest });
            }
        }
        if let Some((_, manifest)) = publication_value {
            if manifest.identity() != *identity {
                return Err(PublicationEvidence::ManifestMismatch);
            }
            matching.push(ResolvedPublication { path: publication, manifest });
        }
        match matching.len() {
            0 => Ok(None),
            1 => Ok(matching.pop()),
            _ => Err(PublicationEvidence::ManifestMismatch),
        }
    }

    fn prepare_capability_links(&self, publication: &Publication) -> Result<(), PublicationEvidence> {
        if self.skills_link.exists() || fs::symlink_metadata(&self.skills_link).is_ok() {
        } else {
            let target = match publication.workspace_image.harness { signal_belief_nexus::Harness::Codex => PathBuf::from("../.belief/current/skills"), signal_belief_nexus::Harness::Claude => self.current.join("skills") };
            symlink(target, &self.skills_link).map_err(|error| io_evidence(&self.skills_link, error))?;
            sync_parent(&self.skills_link).map_err(|error| io_evidence(&self.skills_link, error))?;
        }
        if matches!(publication.workspace_image.harness, signal_belief_nexus::Harness::Claude)
            && !self.settings.exists() && fs::symlink_metadata(&self.settings).is_err() {
            symlink(self.current.join(&self.instructions), &self.settings).map_err(|error| io_evidence(&self.settings, error))?;
            sync_parent(&self.settings).map_err(|error| io_evidence(&self.settings, error))?;
        }
        Ok(())
    }

    fn write_settings(&self, publication: &Publication) -> Result<(), PublicationEvidence> {
        let Some(rendered) = self.rendered_settings(publication)? else { return Ok(()) };
        atomic_file(&self.settings, rendered.as_bytes()).map_err(|error| io_evidence(&self.settings, error))
    }

    fn rendered_settings(&self, publication: &Publication) -> Result<Option<String>, PublicationEvidence> {
        if matches!(publication.workspace_image.harness, signal_belief_nexus::Harness::Claude) { return Ok(None); }
        let existing = match fs::read_to_string(&self.settings) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(io_evidence(&self.settings, error)),
        };
        let mut document = existing.parse::<toml_edit::DocumentMut>()
            .map_err(|_| PublicationEvidence::InvalidSettings)?;
        let key = &publication.workspace_image.layout.instructions_setting.0;
        if key.is_empty() { return Err(PublicationEvidence::InvalidSettings); }
        let value = self.current.join(&self.instructions).to_string_lossy().into_owned();
        let marker = format!("{key}_belief_nexus_managed");
        match document.get(key) {
            None => {
                if document.get(&marker).is_some() { return Err(PublicationEvidence::SettingsConflict); }
                document[key] = toml_edit::value(value.clone());
                document[&marker] = toml_edit::value(value);
            }
            Some(item) => {
                let owned_value = item.as_str() == Some(value.as_str())
                    && document.get(&marker).and_then(toml_edit::Item::as_str) == Some(value.as_str());
                if !owned_value { return Err(PublicationEvidence::SettingsConflict); }
            }
        }
        let rendered = document.to_string();
        if rendered == existing { Ok(None) } else { Ok(Some(rendered)) }
    }

    fn stage_generation(&self, publication: &Publication, generation: &Path, manifest: &PublicationManifest) -> Result<(), PublicationEvidence> {
        let address = self.publication_component(&publication.identity())?;
        let staging = self.generations.join(format!(".staging-{address}-{}", std::process::id()));
        if fs::symlink_metadata(&staging).is_ok() {
            return Err(PublicationEvidence::PermissionDenied);
        }
        fs::create_dir(&staging).map_err(|error| io_evidence(&staging, error))?;
        let result = (|| {
            let instructions = staging.join(&self.instructions);
            if let Some(parent) = instructions.parent() { fs::create_dir_all(parent).map_err(|error| io_evidence(parent, error))?; }
            write_synced(&instructions, publication.workspace_image.content.0.as_bytes()).map_err(|error| io_evidence(&instructions, error))?;
            let skills = staging.join("skills");
            fs::create_dir(&skills).map_err(|error| io_evidence(&skills, error))?;
            for skill in &publication.workspace_image.skill_file_vector {
                let relative = safe_relative(&skill.relative_path.0)?;
                let target = skills.join(relative);
                if let Some(parent) = target.parent() { fs::create_dir_all(parent).map_err(|error| io_evidence(parent, error))?; }
                write_synced(&target, skill.content.0.as_bytes()).map_err(|error| io_evidence(&target, error))?;
            }
            let archived = manifest.signalize().map_err(|_| PublicationEvidence::ManifestMismatch)?;
            write_synced(&staging.join("manifest.signal"), archived.bytes()).map_err(|error| io_evidence(&staging, error))?;
            sync_directory(&staging).map_err(|error| io_evidence(&staging, error))?;
            fs::rename(&staging, generation).map_err(|error| io_evidence(generation, error))?;
            sync_directory(&self.generations).map_err(|error| io_evidence(&self.generations, error))
        })();
        if result.is_err() {
            // The staging name is owned and unreachable from current. Best-effort
            // cleanup cannot change the published generation.
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    fn replace_current(&self, publication: &Publication, generation: &Path) -> Result<(), PublicationEvidence> {
        let observed = self.inspect_current(publication)?.map(|(_, manifest)| manifest.intended_publication.0);
        if observed != publication.prior_publication.0 {
            return Err(PublicationEvidence::PriorRevisionMismatch(RevisionEvidence {
                expected_revision: ExpectedRevision(publication.prior_publication.0.clone()),
                observed_revision: ObservedRevision(observed),
            }));
        }
        let relative = generation.strip_prefix(&self.owned).map_err(|_| boundary(&self.owned, generation))?;
        let address = self.publication_component(&publication.identity())?;
        let temporary = self.owned.join(format!(".current-{address}-{}", std::process::id()));
        symlink(relative, &temporary).map_err(|error| io_evidence(&temporary, error))?;
        fs::rename(&temporary, &self.current).map_err(|error| io_evidence(&self.current, error))?;
        sync_directory(&self.owned).map_err(|error| io_evidence(&self.owned, error))
    }

    fn inspect_current(&self, publication: &Publication) -> Result<Option<(PublicationIdentity, PublicationManifest)>, PublicationEvidence> {
        let (pointer, manifest) = self.inspect_untrusted()?;
        match (pointer, manifest) {
            (None, None) => Ok(None),
            (Some(pointer), Some(manifest)) if pointer.generation_id == manifest.generation_id && pointer.publication_revision == manifest.intended_publication.0 => Ok(Some((pointer, manifest))),
            _ => Err(PublicationEvidence::ManifestMismatch),
        }
        .map_err(|evidence| { let _ = publication; evidence })
    }

    fn inspect_untrusted(&self) -> Result<(Option<PublicationIdentity>, Option<PublicationManifest>), PublicationEvidence> {
        let target = match fs::read_link(&self.current) {
            Ok(target) => target,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((None, None)),
            Err(error) => return Err(io_evidence(&self.current, error)),
        };
        let absolute = self.owned.join(&target);
        let relative = absolute.strip_prefix(&self.generations).map_err(|_| boundary(&self.generations, &absolute))?;
        if relative.components().count() != 1 { return Err(boundary(&self.generations, &absolute)); }
        let (_, manifest) = self.inspect_candidate(&absolute)?.ok_or(PublicationEvidence::ManifestMismatch)?;
        let pointer = manifest.identity();
        Ok((Some(pointer), Some(manifest)))
    }

    fn cleanup(&self, target: &PublicationIdentity) -> Result<CleanupOutcome, PublicationEvidence> {
        let Some(resolved) = self.resolve_publication(target)? else { return Ok(CleanupOutcome::AlreadyAbsent); };
        // Observation alone authorizes nothing. Re-read the owned pointer
        // immediately before the one exact removal and refuse its target.
        let current = self.inspect_untrusted()?.0;
        if current.as_ref() == Some(target) { return Err(PublicationEvidence::OwnedPointerMismatch); }
        fs::remove_dir_all(&resolved.path).map_err(|error| io_evidence(&resolved.path, error))?;
        sync_directory(&self.generations).map_err(|error| io_evidence(&self.generations, error))?;
        Ok(CleanupOutcome::Removed)
    }
}

fn cleanup_failure(workspace: &signal_belief_nexus::Workspace, evidence: PublicationEvidence) -> signal_belief_nexus::FileFailure {
    use signal_belief_nexus::{FileEvidence, FileOperation, RelativePath};
    let file_evidence = match evidence {
        PublicationEvidence::OutsideBoundary(value) => FileEvidence::OutsideBoundary(signal_belief_nexus::OutsideBoundary_Data {
            first_relative_path: RelativePath(value.boundary_root.0.0),
            second_relative_path: RelativePath(value.boundary_candidate.0.0),
        }),
        PublicationEvidence::DestinationUnavailable(value) => FileEvidence::MissingRequired(RelativePath(value.0)),
        PublicationEvidence::PriorRevisionMismatch(value) => FileEvidence::Conflict(signal_belief_nexus::Conflict_Data {
            first_publication_revision_option: value.expected_revision.0,
            second_publication_revision_option: value.observed_revision.0,
        }),
        PublicationEvidence::OwnedPointerMismatch | PublicationEvidence::ManifestMismatch => FileEvidence::Conflict(signal_belief_nexus::Conflict_Data {
            first_publication_revision_option: None, second_publication_revision_option: None,
        }),
        PublicationEvidence::SettingsConflict | PublicationEvidence::InvalidSettings => FileEvidence::Conflict(signal_belief_nexus::Conflict_Data {
            first_publication_revision_option: None, second_publication_revision_option: None,
        }),
        PublicationEvidence::PermissionDenied => FileEvidence::PermissionDenied,
        PublicationEvidence::Translator(value) => FileEvidence::Translator(value),
    };
    signal_belief_nexus::FileFailure { workspace: workspace.clone(), file_operation: FileOperation::Cleaning, file_evidence }
}

fn module_name_from_path(path: &str) -> Option<signal_belief_nexus::ModuleName> {
    let mut parts = path.split('/');
    let kind = match parts.next()? {
        "spirit" => signal_belief_nexus::Kind::Spirit,
        "intent" => signal_belief_nexus::Kind::Intent,
        "vision" => signal_belief_nexus::Kind::Vision,
        "knowledge" => signal_belief_nexus::Kind::Knowledge,
        "role" => signal_belief_nexus::Kind::Role,
        _ => return None,
    };
    let file = parts.next()?;
    if parts.next().is_some() || !file.ends_with(".md") || file.len() == 3 { return None; }
    Some(signal_belief_nexus::ModuleName { kind, subject: signal_belief_nexus::Subject(file[..file.len() - 3].into()) })
}

fn is_git_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn safe_relative(value: &str) -> Result<PathBuf, PublicationEvidence> {
    let path = Path::new(value);
    if path.as_os_str().is_empty() || path.is_absolute() || path.components().any(|part| !matches!(part, Component::Normal(_))) {
        return Err(PublicationEvidence::OutsideBoundary(BoundaryEvidence {
            boundary_root: BoundaryRoot(AbsolutePath(".".into())),
            boundary_candidate: BoundaryCandidate(AbsolutePath(value.into())),
        }));
    }
    Ok(path.to_owned())
}

fn ensure_no_ancestor_symlink(workspace: &Path, target: &Path) -> Result<(), PublicationEvidence> {
    let relative = target.strip_prefix(workspace).map_err(|_| boundary(workspace, target))?;
    let mut cursor = workspace.to_owned();
    for part in relative.components() {
        cursor.push(part);
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) if metadata.file_type().is_symlink() && cursor != target => return Err(boundary(workspace, &cursor)),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(io_evidence(&cursor, error)),
        }
    }
    Ok(())
}

fn boundary(root: &Path, candidate: &Path) -> PublicationEvidence {
    PublicationEvidence::OutsideBoundary(BoundaryEvidence {
        boundary_root: BoundaryRoot(AbsolutePath(root.to_string_lossy().into_owned())),
        boundary_candidate: BoundaryCandidate(AbsolutePath(candidate.to_string_lossy().into_owned())),
    })
}

fn io_evidence(path: &Path, error: std::io::Error) -> PublicationEvidence {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => PublicationEvidence::PermissionDenied,
        std::io::ErrorKind::NotFound => PublicationEvidence::DestinationUnavailable(AbsolutePath(path.to_string_lossy().into_owned())),
        _ => PublicationEvidence::PermissionDenied,
    }
}

fn publication_refusal(publication: &Publication, operation: PublicationOperation, evidence: PublicationEvidence) -> PublicationRefusal {
    PublicationRefusal { workspace: publication.workspace.clone(), publication_operation: operation, publication_evidence: evidence }
}

fn write_synced(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn atomic_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension(format!("belief-{}.tmp", std::process::id()));
    let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
    if let Ok(metadata) = fs::metadata(path) {
        file.set_permissions(metadata.permissions())?;
    }
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    sync_parent(path)
}

fn sync_parent(path: &Path) -> std::io::Result<()> {
    sync_directory(path.parent().expect("owned path has parent"))
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

fn read_manifest(path: &Path) -> Result<PublicationManifest, ()> {
    let bytes = fs::read(path).map_err(|_| ())?;
    Signal::<PublicationManifest>::from(bytes).restore().map_err(|_| ())
}


#[cfg(test)]
mod tests {
    use super::*;
    use meta_signal_belief_nexus::{
        CurrentLink, GenerationsDirectory, InstructionsFile, InstructionsSetting, Layout,
        OwnedRoot, SettingsFile, SkillsDirectory,
    };

    #[test]
    fn frontmatter_placement_is_exact_typed_and_optional() {
        let name = signal_belief_nexus::ModuleName { kind: signal_belief_nexus::Kind::Vision, subject: signal_belief_nexus::Subject("nexus".into()) };
        let path = signal_belief_nexus::RelativePath("vision/nexus.md".into());
        assert_eq!(placement_from_frontmatter("body", &name, &path).unwrap(), None);
        assert_eq!(placement_from_frontmatter("---\nplacement: system-prompt\n---\nbody", &name, &path).unwrap(), Some(signal_belief_nexus::Placement::SystemPrompt));
        assert_eq!(placement_from_frontmatter("---\nplacement: skills\n---\nbody", &name, &path).unwrap(), Some(signal_belief_nexus::Placement::Skills));
        assert!(matches!(placement_from_frontmatter("---\nplacement: elsewhere\n---", &name, &path).unwrap_err().invalid_placement, InvalidPlacement::Unknown(_)));
        assert!(matches!(placement_from_frontmatter("---\nplacement: [skills]\n---", &name, &path).unwrap_err().invalid_placement, InvalidPlacement::NonScalar));
        assert!(matches!(placement_from_frontmatter("---\nplacement: skills\nplacement: system-prompt\n---", &name, &path).unwrap_err().invalid_placement, InvalidPlacement::Duplicate));
    }
    use signal::{ByteViewable, Restorable, Signal, Signalizable};
    use signal_belief_nexus::{
        GenerationId, Kind, ModuleName, PublicationRevision, RelativePath, Repository,
        RepositoryPath, RepositoryState, SourceRevision, Subject, Workspace, WorkspacePath,
    };
    use std::process::Command;

    fn layout() -> Layout {
        Layout {
            owned_root: OwnedRoot(RelativePath(".belief".into())),
            generations_directory: GenerationsDirectory(RelativePath("generations".into())),
            current_link: CurrentLink(RelativePath("current".into())),
            settings_file: SettingsFile(RelativePath(".codex/config.toml".into())),
            instructions_setting: InstructionsSetting("model_instructions_file".into()),
            instructions_file: InstructionsFile(RelativePath("model-instructions.md".into())),
            skills_directory: SkillsDirectory(RelativePath(".agents/skills".into())),
        }
    }

    fn claude_layout() -> Layout {
        Layout {
            owned_root: OwnedRoot(RelativePath(".belief".into())),
            generations_directory: GenerationsDirectory(RelativePath("generations".into())),
            current_link: CurrentLink(RelativePath("current".into())),
            settings_file: SettingsFile(RelativePath("CLAUDE.md".into())),
            instructions_setting: InstructionsSetting("unused-for-claude".into()),
            instructions_file: InstructionsFile(RelativePath("CLAUDE.md".into())),
            skills_directory: SkillsDirectory(RelativePath(".claude/skills".into())),
        }
    }

    fn publication(root: &Path, generation: &str, prior: Option<i64>, intended: i64, instruction: &str, skills: Vec<SkillFile>) -> Publication {
        Publication {
            workspace: Workspace(WorkspacePath(root.to_string_lossy().into_owned())),
            generation_id: GenerationId(generation.into()),
            prior_publication: PriorPublication(prior.map(PublicationRevision)),
            intended_publication: IntendedPublication(PublicationRevision(intended)),
            workspace_image: WorkspaceImage { harness: signal_belief_nexus::Harness::Codex, layout: layout(), content: Content(instruction.into()), skill_file_vector: skills },
            repository_states: RepositoryStates(vec![RepositoryState {
                repository: Repository(RepositoryPath("/source".into())),
                source_revision: SourceRevision(format!("revision-{intended}")),
            }]),
            found_modules: FoundModules(vec![ModuleName { kind: Kind::Vision, subject: Subject("nexus".into()) }]),
            missing_modules: MissingModules(vec![],),
            retention_policy: meta_signal_belief_nexus::RetentionPolicy::KeepAll,
        }
    }

    fn publication_component_for(generation: &str, revision: i64) -> String {
        let hex: String = generation.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect();
        format!("publication-{revision}-{hex}")
    }

    fn write_publication_directory(publication: &Publication, generation: &Path) {
        fs::create_dir_all(generation.join("skills")).unwrap();
        fs::write(generation.join("model-instructions.md"), &publication.workspace_image.content.0).unwrap();
        let manifest = publication.manifest().signalize().unwrap();
        fs::write(generation.join("manifest.signal"), manifest.bytes()).unwrap();
    }

    fn install_legacy(publication: &Publication, current: bool) -> PathBuf {
        let workspace = Path::new(&(publication.workspace.0).0);
        let owned = workspace.join(".belief");
        let generation = owned.join("generations").join(&publication.generation_id.0);
        write_publication_directory(publication, &generation);
        if current {
            let current_path = owned.join("current");
            if fs::symlink_metadata(&current_path).is_ok() { fs::remove_file(&current_path).unwrap(); }
            symlink(PathBuf::from("generations").join(&publication.generation_id.0), current_path).unwrap();
        }
        generation
    }

    fn run_git(root: &Path, args: &[&str]) {
        let status = Command::new("git").arg("-C").arg(root).args(args).status().expect("run git fixture");
        assert!(status.success(), "git fixture failed: {args:?}");
    }

    #[test]
    fn publication_carries_prior_and_intended_revision_with_owned_layout() {
        let publication = Publication {
            workspace: Workspace(WorkspacePath("/tmp/work".into())),
            generation_id: GenerationId("sha256:generation".into()),
            prior_publication: PriorPublication(Some(PublicationRevision(3))),
            intended_publication: IntendedPublication(PublicationRevision(4)),
            workspace_image: WorkspaceImage { harness: signal_belief_nexus::Harness::Codex, layout: layout(), content: Content("instructions".into()), skill_file_vector: vec![] },
            repository_states: RepositoryStates(vec![]),
            found_modules: FoundModules(vec![]),
            missing_modules: MissingModules(vec![ModuleName { kind: Kind::Vision, subject: Subject("missing".into()) }]),
            retention_policy: meta_signal_belief_nexus::RetentionPolicy::KeepAll,
        };
        let query = Query::Publish(publication);
        let archived = query.signalize().expect("archive emission");
        assert_eq!(Signal::<Query>::from(archived.bytes().to_vec()).restore().unwrap(), query);
    }

    #[test]
    fn inspection_returns_pointer_and_manifest_as_separate_recovery_evidence() {
        let response = Response::Inspected(PublicationInspection {
            workspace: Workspace(WorkspacePath("/tmp/work".into())),
            publication_identity_option: Some(PublicationIdentity { generation_id: GenerationId("sha256:generation".into()), publication_revision: PublicationRevision(4) }),
            publication_manifest_option: Some(PublicationManifest {
                generation_id: GenerationId("sha256:generation".into()),
                intended_publication: IntendedPublication(PublicationRevision(4)),
                repository_states: RepositoryStates(vec![]),
                found_modules: FoundModules(vec![]),
                missing_modules: MissingModules(vec![]),
            }),
        });
        let archived = response.signalize().expect("archive inspection");
        assert_eq!(Signal::<Response>::from(archived.bytes().to_vec()).restore().unwrap(), response);
    }

    #[test]
    fn snapshot_reads_one_committed_main_tree_and_ignores_worktree_changes() {
        let repository = tempfile::tempdir().unwrap();
        run_git(repository.path(), &["init", "-b", "main"]);
        run_git(repository.path(), &["config", "user.email", "belief@example.test"]);
        run_git(repository.path(), &["config", "user.name", "Belief Test"]);
        fs::create_dir(repository.path().join("vision")).unwrap();
        fs::create_dir(repository.path().join("nested")).unwrap();
        fs::write(repository.path().join("vision/nexus.md"), "committed\n").unwrap();
        fs::write(repository.path().join("nested/ignored.md"), "ignored\n").unwrap();
        run_git(repository.path(), &["add", "."]);
        run_git(repository.path(), &["commit", "-m", "fixture"]);
        fs::write(repository.path().join("vision/nexus.md"), "working tree\n").unwrap();

        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let response = emission.execute(Query::Snapshot(Repository(RepositoryPath(repository.path().to_string_lossy().into_owned()))));
        let Response::Snapshotted(snapshot) = response else { panic!("snapshot refused: {response:?}") };
        assert!(is_git_object_id(&snapshot.source_revision.0));
        assert_eq!(snapshot.module_source_vector.len(), 1);
        assert_eq!(snapshot.module_source_vector[0].content.0, "committed\n");
        assert_eq!(snapshot.module_source_vector[0].relative_path.0, "vision/nexus.md");
    }

    #[test]
    fn publication_preserves_settings_and_swaps_one_owned_pointer_atomically() {
        let workspace = tempfile::tempdir().unwrap();
        fs::create_dir(workspace.path().join(".codex")).unwrap();
        fs::write(workspace.path().join(".codex/config.toml"), "model = \"gpt-test\"\n[sandbox]\nmode = \"strict\"\n").unwrap();
        fs::write(workspace.path().join("unrelated.txt"), "keep").unwrap();
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let first = publication(
            workspace.path(),
            "generation-one",
            None,
            1,
            "first instructions",
            vec![SkillFile { relative_path: RelativePath("nexus/SKILL.md".into()), content: Content("first skill".into()) }],
        );
        assert!(matches!(emission.execute(Query::Publish(first)), Response::Published(_)));
        let first_component = publication_component_for("generation-one", 1);
        assert_eq!(fs::read_link(workspace.path().join(".belief/current")).unwrap(), PathBuf::from("generations").join(&first_component));
        assert_eq!(fs::read_link(workspace.path().join(".agents/skills")).unwrap(), PathBuf::from("../.belief/current/skills"));
        assert_eq!(fs::read_to_string(workspace.path().join("unrelated.txt")).unwrap(), "keep");
        let settings = fs::read_to_string(workspace.path().join(".codex/config.toml")).unwrap();
        assert!(settings.contains("model = \"gpt-test\""));
        assert!(settings.contains("mode = \"strict\""));
        assert!(settings.contains(&format!("model_instructions_file = \"{}/.belief/current/model-instructions.md\"", workspace.path().display())));

        let invalid = publication(
            workspace.path(), "generation-invalid", Some(1), 2, "invalid",
            vec![SkillFile { relative_path: RelativePath("../escape".into()), content: Content("escape".into()) }],
        );
        assert!(matches!(emission.execute(Query::Publish(invalid)), Response::EmissionRefused(_)));
        assert_eq!(fs::read_link(workspace.path().join(".belief/current")).unwrap(), PathBuf::from("generations").join(&first_component));
        assert!(!workspace.path().join(".belief/generations").join(publication_component_for("generation-invalid", 2)).exists());

        let second = publication(workspace.path(), "generation-two", Some(1), 2, "second instructions", vec![]);
        assert!(matches!(emission.execute(Query::Publish(second)), Response::Published(_)));
        let second_component = publication_component_for("generation-two", 2);
        assert_eq!(fs::read_link(workspace.path().join(".belief/current")).unwrap(), PathBuf::from("generations").join(&second_component));
        assert_eq!(fs::read_to_string(workspace.path().join(".belief/generations").join(first_component).join("model-instructions.md")).unwrap(), "first instructions");

        let inspected = emission.execute(Query::InspectPublication(InspectionRequest {
            workspace: Workspace(WorkspacePath(workspace.path().to_string_lossy().into_owned())),
            publication_identity: PublicationIdentity { generation_id: GenerationId("generation-two".into()), publication_revision: PublicationRevision(2) },
            layout: layout(),
        }));
        let Response::Inspected(observed) = inspected else { panic!("inspection refused: {inspected:?}") };
        assert_eq!(observed.publication_identity_option.as_ref().unwrap().generation_id, GenerationId("generation-two".into()));
        let manifest = observed.publication_manifest_option.unwrap();
        assert_eq!(manifest.generation_id, GenerationId("generation-two".into()));
        assert_eq!(manifest.intended_publication, IntendedPublication(PublicationRevision(2)));
        assert_eq!(manifest.repository_states.0.len(), 1);
        assert_eq!(manifest.found_modules.0.len(), 1);
    }

    #[test]
    fn claude_publication_installs_only_owned_capability_links() {
        let workspace = tempfile::tempdir().unwrap();
        fs::write(workspace.path().join("unrelated.txt"), "keep").unwrap();
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let mut first = publication(
            workspace.path(), "claude-one", None, 1, "claude instructions",
            vec![SkillFile { relative_path: RelativePath("witness/SKILL.md".into()), content: Content("claude skill".into()) }],
        );
        first.workspace_image.harness = signal_belief_nexus::Harness::Claude;
        first.workspace_image.layout = claude_layout();
        assert!(matches!(emission.execute(Query::Publish(first)), Response::Published(_)));
        assert_eq!(fs::read_link(workspace.path().join("CLAUDE.md")).unwrap(), workspace.path().join(".belief/current/CLAUDE.md"));
        assert_eq!(fs::read_link(workspace.path().join(".claude/skills")).unwrap(), workspace.path().join(".belief/current/skills"));
        assert_eq!(fs::read_to_string(workspace.path().join("CLAUDE.md")).unwrap(), "claude instructions");
        assert_eq!(fs::read_to_string(workspace.path().join(".claude/skills/witness/SKILL.md")).unwrap(), "claude skill");
        assert_eq!(fs::read_to_string(workspace.path().join("unrelated.txt")).unwrap(), "keep");

        let foreign = tempfile::tempdir().unwrap();
        fs::write(foreign.path().join("CLAUDE.md"), "foreign").unwrap();
        let mut refused = publication(foreign.path(), "claude-one", None, 1, "ours", vec![]);
        refused.workspace_image.harness = signal_belief_nexus::Harness::Claude;
        refused.workspace_image.layout = claude_layout();
        assert!(matches!(emission.execute(Query::Publish(refused)), Response::EmissionRefused(_)));
        assert_eq!(fs::read_to_string(foreign.path().join("CLAUDE.md")).unwrap(), "foreign");
        assert!(!foreign.path().join(".belief/current").exists());

        let foreign_skills = tempfile::tempdir().unwrap();
        fs::create_dir_all(foreign_skills.path().join(".claude/skills")).unwrap();
        fs::write(foreign_skills.path().join(".claude/skills/keep"), "foreign").unwrap();
        let mut refused = publication(foreign_skills.path(), "claude-one", None, 1, "ours", vec![]);
        refused.workspace_image.harness = signal_belief_nexus::Harness::Claude;
        refused.workspace_image.layout = claude_layout();
        assert!(matches!(emission.execute(Query::Publish(refused)), Response::EmissionRefused(_)));
        assert_eq!(fs::read_to_string(foreign_skills.path().join(".claude/skills/keep")).unwrap(), "foreign");
    }

    #[test]
    fn publication_preserves_foreign_current_and_settings_ownership() {
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();

        let regular = tempfile::tempdir().unwrap();
        fs::create_dir_all(regular.path().join(".belief")).unwrap();
        fs::write(regular.path().join(".belief/current"), "foreign").unwrap();
        let response = emission.execute(Query::Publish(publication(regular.path(), "one", None, 1, "instructions", vec![])));
        assert!(matches!(response, Response::EmissionRefused(_)));
        assert_eq!(fs::read_to_string(regular.path().join(".belief/current")).unwrap(), "foreign");

        let linked = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(linked.path().join(".belief")).unwrap();
        symlink(outside.path(), linked.path().join(".belief/current")).unwrap();
        let response = emission.execute(Query::Publish(publication(linked.path(), "one", None, 1, "instructions", vec![])));
        assert!(matches!(response, Response::EmissionRefused(_)));
        assert_eq!(fs::read_link(linked.path().join(".belief/current")).unwrap(), outside.path());

        for existing in [
            "model_instructions_file = \".belief/current/model-instructions.md\"\n",
            "model_instructions_file = \"foreign.md\"\n",
            "model_instructions_file = [\n",
        ] {
            let workspace = tempfile::tempdir().unwrap();
            fs::create_dir_all(workspace.path().join(".codex")).unwrap();
            fs::write(workspace.path().join(".codex/config.toml"), existing).unwrap();
            let response = emission.execute(Query::Publish(publication(workspace.path(), "one", None, 1, "instructions", vec![])));
            assert!(matches!(response, Response::EmissionRefused(_)));
            assert_eq!(fs::read_to_string(workspace.path().join(".codex/config.toml")).unwrap(), existing);
            assert!(!workspace.path().join(".belief/current").exists());
        }
    }

    #[test]
    fn settings_merge_preserves_unrelated_nested_toml_exactly() {
        let workspace = tempfile::tempdir().unwrap();
        fs::create_dir_all(workspace.path().join(".codex")).unwrap();
        let unrelated = "model = \"gpt-test\" # keep comment\n[nested]\narray = [1, 2, 3]\ninline = { answer = 42 }\n";
        fs::write(workspace.path().join(".codex/config.toml"), unrelated).unwrap();
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        assert!(matches!(emission.execute(Query::Publish(publication(workspace.path(), "one", None, 1, "instructions", vec![]))), Response::Published(_)));
        let rendered = fs::read_to_string(workspace.path().join(".codex/config.toml")).unwrap();
        assert!(rendered.contains("model = \"gpt-test\" # keep comment\n"));
        assert!(rendered.contains("[nested]\narray = [1, 2, 3]\ninline = { answer = 42 }\n"));
        let before = unrelated.parse::<toml_edit::DocumentMut>().unwrap();
        let after = rendered.parse::<toml_edit::DocumentMut>().unwrap();
        assert_eq!(before["model"].as_str(), after["model"].as_str());
        assert_eq!(before["nested"].to_string(), after["nested"].to_string());
    }

    #[test]
    fn publication_refuses_unowned_skill_destination_and_owned_root_symlink() {
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let occupied = tempfile::tempdir().unwrap();
        fs::create_dir_all(occupied.path().join(".agents/skills")).unwrap();
        fs::write(occupied.path().join(".agents/skills/unrelated"), "keep").unwrap();
        let response = emission.execute(Query::Publish(publication(occupied.path(), "generation", None, 1, "instructions", vec![])));
        assert!(matches!(response, Response::EmissionRefused(_)));
        assert_eq!(fs::read_to_string(occupied.path().join(".agents/skills/unrelated")).unwrap(), "keep");
        assert!(!occupied.path().join(".belief/current").exists());

        let linked = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), linked.path().join(".belief")).unwrap();
        let response = emission.execute(Query::Publish(publication(linked.path(), "generation", None, 1, "instructions", vec![])));
        assert!(matches!(response, Response::EmissionRefused(_)));
        assert!(!outside.path().join("current").exists());
    }

    #[test]
    fn cleanup_removes_only_the_exact_noncurrent_owned_generation_idempotently() {
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let first = publication(workspace.path(), "first", None, 1, "first", vec![]);
        assert!(matches!(emission.execute(Query::Publish(first)), Response::Published(_)));
        let second = publication(workspace.path(), "second", Some(1), 2, "second", vec![]);
        assert!(matches!(emission.execute(Query::Publish(second)), Response::Published(_)));

        let request = CleanupRequest {
            workspace: Workspace(WorkspacePath(workspace.path().to_string_lossy().into_owned())),
            publication_identity: PublicationIdentity { generation_id: GenerationId("first".into()), publication_revision: PublicationRevision(1) }, layout: layout(),
        };
        assert!(matches!(emission.execute(Query::Clean(request.clone())), Response::Cleaned(CleanupState {
            cleanup_outcome: CleanupOutcome::Removed, ..
        })));
        assert!(matches!(emission.execute(Query::Clean(request)), Response::Cleaned(CleanupState {
            cleanup_outcome: CleanupOutcome::AlreadyAbsent, ..
        })));
        let current = CleanupRequest {
            workspace: Workspace(WorkspacePath(workspace.path().to_string_lossy().into_owned())),
            publication_identity: PublicationIdentity { generation_id: GenerationId("second".into()), publication_revision: PublicationRevision(2) }, layout: layout(),
        };
        assert!(matches!(emission.execute(Query::Clean(current)), Response::CleanupRefused(signal_belief_nexus::FileFailure {
            file_operation: signal_belief_nexus::FileOperation::Cleaning, ..
        })));
        assert!(workspace.path().join(".belief/generations").join(publication_component_for("second", 2)).is_dir());
    }

    #[test]
    fn same_generation_publications_coexist_and_cleanup_targets_the_exact_pair() {
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let first = publication(workspace.path(), "g", None, 1, "same", vec![]);
        let legacy = install_legacy(&first, true);
        let second = publication(workspace.path(), "g", Some(1), 2, "same", vec![]);
        let response = emission.execute(Query::Publish(second));
        assert!(matches!(response, Response::Published(PublicationState {
            publication_identity: PublicationIdentity { generation_id: GenerationId(ref value), publication_revision: PublicationRevision(2) }, ..
        }) if value == "g"));
        let current = fs::read_link(workspace.path().join(".belief/current")).unwrap();
        assert_eq!(current, PathBuf::from("generations").join(publication_component_for("g", 2)));
        assert!(legacy.is_dir());

        let cleaned = emission.execute(Query::Clean(CleanupRequest {
            workspace: Workspace(WorkspacePath(workspace.path().to_string_lossy().into_owned())),
            publication_identity: PublicationIdentity { generation_id: GenerationId("g".into()), publication_revision: PublicationRevision(1) },
            layout: layout(),
        }));
        assert!(matches!(cleaned, Response::Cleaned(CleanupState { cleanup_outcome: CleanupOutcome::Removed, .. })));
        assert!(!legacy.exists());
        assert!(workspace.path().join(".belief/generations").join(publication_component_for("g", 2)).is_dir());
    }

    #[test]
    fn exact_legacy_pending_publication_is_reused_without_rewrite() {
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let first = publication(workspace.path(), "old", None, 1, "old", vec![]);
        install_legacy(&first, true);
        let pending = publication(workspace.path(), "same", Some(1), 2, "same", vec![]);
        let legacy = install_legacy(&pending, false);
        let before = fs::metadata(&legacy).unwrap().modified().unwrap();
        assert!(matches!(emission.execute(Query::Publish(pending)), Response::Published(_)));
        assert_eq!(fs::read_link(workspace.path().join(".belief/current")).unwrap(), PathBuf::from("generations/same"));
        assert_eq!(fs::metadata(&legacy).unwrap().modified().unwrap(), before);
        assert!(!workspace.path().join(".belief/generations").join(publication_component_for("same", 2)).exists());
    }

    #[test]
    fn matching_legacy_and_publication_addresses_refuse_without_changing_current() {
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let prior = publication(workspace.path(), "old", None, 1, "old", vec![]);
        install_legacy(&prior, true);
        let requested = publication(workspace.path(), "g", Some(1), 2, "same", vec![]);
        install_legacy(&requested, false);
        let canonical = workspace.path().join(".belief/generations").join(publication_component_for("g", 2));
        write_publication_directory(&requested, &canonical);
        let before = fs::read_link(workspace.path().join(".belief/current")).unwrap();
        assert!(matches!(emission.execute(Query::Publish(requested)), Response::EmissionRefused(_)));
        assert_eq!(fs::read_link(workspace.path().join(".belief/current")).unwrap(), before);
    }

    #[test]
    fn canonical_path_owned_by_another_legacy_identity_refuses_without_effect() {
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let prior = publication(workspace.path(), "old", None, 1, "old", vec![]);
        install_legacy(&prior, true);
        let occupied_name = publication_component_for("g", 2);
        let occupied = publication(workspace.path(), &occupied_name, Some(1), 1, "occupied", vec![]);
        install_legacy(&occupied, false);
        let before = fs::read_link(workspace.path().join(".belief/current")).unwrap();
        let requested = publication(workspace.path(), "g", Some(1), 2, "same", vec![]);
        assert!(matches!(emission.execute(Query::Publish(requested)), Response::EmissionRefused(_)));
        assert_eq!(fs::read_link(workspace.path().join(".belief/current")).unwrap(), before);
    }

    #[test]
    fn invalid_requested_canonical_manifest_refuses_before_capability_effects() {
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let requested = publication(workspace.path(), "g", None, 1, "same", vec![]);
        let canonical = workspace.path().join(".belief/generations").join(publication_component_for("g", 1));
        fs::create_dir_all(&canonical).unwrap();
        fs::write(canonical.join("manifest.signal"), b"invalid").unwrap();
        assert!(matches!(emission.execute(Query::Publish(requested)), Response::EmissionRefused(_)));
        assert!(!workspace.path().join(".belief/current").exists());
        assert!(!workspace.path().join(".agents/skills").exists());
        assert!(!workspace.path().join(".codex/config.toml").exists());
    }

    #[test]
    fn duplicate_skill_destination_refuses_before_capability_effects() {
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let duplicate_skills = vec![
            SkillFile { relative_path: RelativePath("witness/SKILL.md".into()), content: Content("first".into()) },
            SkillFile { relative_path: RelativePath("witness/SKILL.md".into()), content: Content("second".into()) },
        ];

        let fresh = tempfile::tempdir().unwrap();
        let refused = publication(fresh.path(), "duplicate", None, 1, "instructions", duplicate_skills.clone());
        assert!(matches!(emission.execute(Query::Publish(refused)), Response::EmissionRefused(EmissionRefusal::Publication(PublicationRefusal {
            publication_operation: PublicationOperation::StagingGeneration,
            publication_evidence: PublicationEvidence::PermissionDenied,
            ..
        }))));
        assert!(fs::read_dir(fresh.path()).unwrap().next().is_none());

        let existing = tempfile::tempdir().unwrap();
        fs::create_dir_all(existing.path().join(".codex")).unwrap();
        let settings = b"model = \"gpt-test\" # exact\n";
        fs::write(existing.path().join(".codex/config.toml"), settings).unwrap();
        fs::write(existing.path().join("keep"), b"foreign").unwrap();
        let refused = publication(existing.path(), "duplicate", None, 1, "instructions", duplicate_skills);
        assert!(matches!(emission.execute(Query::Publish(refused)), Response::EmissionRefused(_)));
        assert_eq!(fs::read(existing.path().join(".codex/config.toml")).unwrap(), settings);
        assert_eq!(fs::read(existing.path().join("keep")).unwrap(), b"foreign");
        assert!(!existing.path().join(".belief").exists());
        assert!(!existing.path().join(".agents").exists());

        let distinct = tempfile::tempdir().unwrap();
        let valid = publication(distinct.path(), "distinct", None, 1, "instructions", vec![
            SkillFile { relative_path: RelativePath("first/SKILL.md".into()), content: Content("first".into()) },
            SkillFile { relative_path: RelativePath("second/SKILL.md".into()), content: Content("second".into()) },
        ]);
        assert!(matches!(emission.execute(Query::Publish(valid)), Response::Published(_)));
        assert_eq!(fs::read_to_string(distinct.path().join(".agents/skills/first/SKILL.md")).unwrap(), "first");
        assert_eq!(fs::read_to_string(distinct.path().join(".agents/skills/second/SKILL.md")).unwrap(), "second");
    }

    #[test]
    fn ancestor_skill_destinations_refuse_before_capability_effects_in_either_order() {
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        for (generation, paths) in [
            ("file-first", ["witness", "witness/SKILL.md"]),
            ("directory-first", ["witness/SKILL.md", "witness"]),
        ] {
            let workspace = tempfile::tempdir().unwrap();
            let refused = publication(workspace.path(), generation, None, 1, "instructions", paths.into_iter().map(|path|
                SkillFile { relative_path: RelativePath(path.into()), content: Content(path.into()) }
            ).collect());
            assert!(matches!(emission.execute(Query::Publish(refused)), Response::EmissionRefused(EmissionRefusal::Publication(PublicationRefusal {
                publication_operation: PublicationOperation::StagingGeneration,
                publication_evidence: PublicationEvidence::PermissionDenied,
                ..
            }))));
            assert!(fs::read_dir(workspace.path()).unwrap().next().is_none());
        }

        let existing = tempfile::tempdir().unwrap();
        fs::create_dir_all(existing.path().join(".codex")).unwrap();
        let settings = b"model = \"gpt-test\" # exact\n";
        fs::write(existing.path().join(".codex/config.toml"), settings).unwrap();
        fs::write(existing.path().join("keep"), b"foreign").unwrap();
        let refused = publication(existing.path(), "ancestor", None, 1, "instructions", vec![
            SkillFile { relative_path: RelativePath("witness".into()), content: Content("file".into()) },
            SkillFile { relative_path: RelativePath("witness/SKILL.md".into()), content: Content("nested".into()) },
        ]);
        assert!(matches!(emission.execute(Query::Publish(refused)), Response::EmissionRefused(_)));
        assert_eq!(fs::read(existing.path().join(".codex/config.toml")).unwrap(), settings);
        assert_eq!(fs::read(existing.path().join("keep")).unwrap(), b"foreign");
        assert!(!existing.path().join(".belief").exists());
        assert!(!existing.path().join(".agents").exists());
    }

    #[test]
    fn canonical_codec_round_trips_utf8_and_is_pair_injective() {
        let workspace = tempfile::tempdir().unwrap();
        let paths = PublicationPaths::new(workspace.path(), &layout()).unwrap();
        let unicode = PublicationIdentity { generation_id: GenerationId("信念".into()), publication_revision: PublicationRevision(2) };
        assert_eq!(paths.publication_component(&unicode).unwrap(), "publication-2-e4bfa1e5bfb5");
        assert!(paths.publication_component(&PublicationIdentity {
            generation_id: GenerationId("g".into()),
            publication_revision: PublicationRevision(0),
        }).is_err());
        let pairs = [
            PublicationIdentity { generation_id: GenerationId("g".into()), publication_revision: PublicationRevision(2) },
            PublicationIdentity { generation_id: GenerationId("g2".into()), publication_revision: PublicationRevision(2) },
            PublicationIdentity { generation_id: GenerationId("g".into()), publication_revision: PublicationRevision(23) },
            PublicationIdentity { generation_id: GenerationId("publication-2-67".into()), publication_revision: PublicationRevision(1) },
        ];
        let names: std::collections::HashSet<_> = pairs.iter().map(|value| paths.publication_component(value).unwrap()).collect();
        assert_eq!(names.len(), pairs.len());
    }

    #[test]
    fn canonical_looking_legacy_name_remains_legacy() {
        let emission = FileEmission::new(PathBuf::from("git")).unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let publication = publication(workspace.path(), "publication-2-67", None, 1, "legacy", vec![]);
        let legacy = install_legacy(&publication, false);
        assert!(matches!(emission.execute(Query::Publish(publication)), Response::Published(_)));
        assert_eq!(fs::read_link(workspace.path().join(".belief/current")).unwrap(), PathBuf::from("generations/publication-2-67"));
        assert!(legacy.is_dir());
    }
}
