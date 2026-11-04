#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    clippy::type_complexity
)]
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
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct PublicationState {
    pub workspace: signal_belief_nexus::Workspace,
    pub generation_id: signal_belief_nexus::GenerationId,
    pub publication_revision: signal_belief_nexus::PublicationRevision,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct InspectionRequest {
    pub workspace: signal_belief_nexus::Workspace,
    pub layout: meta_signal_belief_nexus::Layout,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct OwnedPointer {
    pub generation_id: signal_belief_nexus::GenerationId,
    pub publication_revision: signal_belief_nexus::PublicationRevision,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct PublicationInspection {
    pub workspace: signal_belief_nexus::Workspace,
    pub owned_pointer_option: std::option::Option<OwnedPointer>,
    pub publication_manifest_option: std::option::Option<PublicationManifest>,
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
pub enum SourceEvidence {
    NotRepository,
    MainUnavailable,
    RevisionUnavailable,
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
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Response {
    Snapshotted(SourceSnapshot),
    Published(PublicationState),
    Inspected(PublicationInspection),
    EmissionRefused(EmissionRefusal),
}

/// The authored schema whose digest identifies this internal effect contract.
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
/// The executable and retention count are supplied configuration. The Nexus
/// sends only typed `Query` values and never observes command lines or paths
/// that have not passed through this boundary.
#[derive(Clone, Debug)]
pub struct FileEmission {
    git_executable: PathBuf,
    retained_generations: usize,
}

impl FileEmission {
    pub fn new(git_executable: PathBuf, retained_generations: usize) -> Result<Self, String> {
        if retained_generations == 0 {
            return Err("retained_generations must be positive".into());
        }
        Ok(Self { git_executable, retained_generations })
    }

    pub fn execute(&self, query: Query) -> Response {
        match query {
            Query::Snapshot(repository) => self.snapshot(repository),
            Query::Publish(publication) => self.publish(publication),
            Query::InspectPublication(request) => self.inspect(request),
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
            modules.push(ModuleSource {
                module_name,
                relative_path: signal_belief_nexus::RelativePath(path.into()),
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

        paths.prepare_skills_link(publication)
            .map_err(|evidence| publication_refusal(publication, PublicationOperation::StagingGeneration, evidence))?;
        paths.write_settings(publication)
            .map_err(|evidence| publication_refusal(publication, PublicationOperation::StagingSettings, evidence))?;

        let generation = paths.generation(&publication.generation_id)
            .map_err(|evidence| publication_refusal(publication, operation.clone(), evidence))?;
        let intended_manifest = publication.manifest();
        if generation.exists() {
            let existing = read_manifest(&generation.join("manifest.signal"))
                .map_err(|_| publication_refusal(publication, PublicationOperation::InspectingManifest, PublicationEvidence::ManifestMismatch))?;
            if existing != intended_manifest {
                return Err(publication_refusal(publication, PublicationOperation::InspectingManifest, PublicationEvidence::ManifestMismatch));
            }
        } else {
            paths.stage_generation(publication, &generation, &intended_manifest)
                .map_err(|evidence| publication_refusal(publication, operation.clone(), evidence))?;
        }

        paths.replace_current(publication, &generation)
            .map_err(|evidence| publication_refusal(publication, PublicationOperation::ReplacingPointer, evidence))?;
        paths.retain(self.retained_generations, &publication.generation_id);

        Ok(PublicationState {
            workspace: publication.workspace.clone(),
            generation_id: publication.generation_id.clone(),
            publication_revision: publication.intended_publication.0.clone(),
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
        match paths.inspect_untrusted() {
            Ok((pointer, manifest)) => Response::Inspected(PublicationInspection {
                workspace: request.workspace,
                owned_pointer_option: pointer,
                publication_manifest_option: manifest,
            }),
            Err(evidence) => Response::EmissionRefused(EmissionRefusal::Publication(PublicationRefusal {
                workspace: request.workspace,
                publication_operation: PublicationOperation::InspectingManifest,
                publication_evidence: evidence,
            })),
        }
    }
}

impl Publication {
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
        if let Ok(metadata) = fs::symlink_metadata(&self.settings) {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(boundary(&self.workspace, &self.settings));
            }
        }
        if let Ok(metadata) = fs::symlink_metadata(&self.skills_link) {
            if !metadata.file_type().is_symlink() || fs::read_link(&self.skills_link).ok().as_deref() != Some(Path::new("../.belief/current/skills")) {
                return Err(boundary(&self.workspace, &self.skills_link));
            }
        }
        for skill in &publication.workspace_image.skill_file_vector {
            let relative = safe_relative(&skill.relative_path.0)?;
            let candidate = self.generations.join(&publication.generation_id.0).join("skills").join(relative);
            if !candidate.starts_with(&self.generations) {
                return Err(boundary(&self.generations, &candidate));
            }
        }
        let _ = self.generation(&publication.generation_id)?;
        Ok(())
    }

    fn generation(&self, generation: &signal_belief_nexus::GenerationId) -> Result<PathBuf, PublicationEvidence> {
        if generation.0.is_empty() || generation.0.contains('/') || generation.0.contains('\\') || generation.0 == "." || generation.0 == ".." {
            return Err(boundary(&self.generations, &self.generations.join(&generation.0)));
        }
        Ok(self.generations.join(&generation.0))
    }

    fn prepare_skills_link(&self, publication: &Publication) -> Result<(), PublicationEvidence> {
        if self.skills_link.exists() || fs::symlink_metadata(&self.skills_link).is_ok() {
            return Ok(());
        }
        symlink("../.belief/current/skills", &self.skills_link)
            .map_err(|error| io_evidence(&self.skills_link, error))?;
        sync_parent(&self.skills_link).map_err(|error| io_evidence(&self.skills_link, error))?;
        let _ = publication;
        Ok(())
    }

    fn write_settings(&self, publication: &Publication) -> Result<(), PublicationEvidence> {
        let existing = match fs::read_to_string(&self.settings) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(io_evidence(&self.settings, error)),
        };
        let mut document = existing.parse::<toml_edit::DocumentMut>()
            .map_err(|_| PublicationEvidence::PermissionDenied)?;
        let key = &publication.workspace_image.layout.instructions_setting.0;
        if key.is_empty() { return Err(PublicationEvidence::PermissionDenied); }
        let value = format!("{}/{}", relative_from(&self.workspace, &self.current), self.instructions.to_string_lossy());
        document[key] = toml_edit::value(value);
        let rendered = document.to_string();
        if rendered == existing { return Ok(()); }
        atomic_file(&self.settings, rendered.as_bytes()).map_err(|error| io_evidence(&self.settings, error))
    }

    fn stage_generation(&self, publication: &Publication, generation: &Path, manifest: &PublicationManifest) -> Result<(), PublicationEvidence> {
        let staging = self.generations.join(format!(".staging-{}-{}", publication.generation_id.0, std::process::id()));
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
        let relative = generation.strip_prefix(&self.owned).map_err(|_| boundary(&self.owned, generation))?;
        let temporary = self.owned.join(format!(".current-{}-{}", publication.generation_id.0, std::process::id()));
        symlink(relative, &temporary).map_err(|error| io_evidence(&temporary, error))?;
        fs::rename(&temporary, &self.current).map_err(|error| io_evidence(&self.current, error))?;
        sync_directory(&self.owned).map_err(|error| io_evidence(&self.owned, error))
    }

    fn inspect_current(&self, publication: &Publication) -> Result<Option<(OwnedPointer, PublicationManifest)>, PublicationEvidence> {
        let (pointer, manifest) = self.inspect_untrusted()?;
        match (pointer, manifest) {
            (None, None) => Ok(None),
            (Some(pointer), Some(manifest)) if pointer.generation_id == manifest.generation_id && pointer.publication_revision == manifest.intended_publication.0 => Ok(Some((pointer, manifest))),
            _ => Err(PublicationEvidence::ManifestMismatch),
        }
        .map_err(|evidence| { let _ = publication; evidence })
    }

    fn inspect_untrusted(&self) -> Result<(Option<OwnedPointer>, Option<PublicationManifest>), PublicationEvidence> {
        let target = match fs::read_link(&self.current) {
            Ok(target) => target,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((None, None)),
            Err(error) => return Err(io_evidence(&self.current, error)),
        };
        let absolute = self.owned.join(&target);
        let relative = absolute.strip_prefix(&self.generations).map_err(|_| boundary(&self.generations, &absolute))?;
        if relative.components().count() != 1 { return Err(boundary(&self.generations, &absolute)); }
        let manifest = read_manifest(&absolute.join("manifest.signal")).map_err(|_| PublicationEvidence::ManifestMismatch)?;
        let pointer = OwnedPointer { generation_id: manifest.generation_id.clone(), publication_revision: manifest.intended_publication.0.clone() };
        Ok((Some(pointer), Some(manifest)))
    }

    fn retain(&self, retained: usize, current: &signal_belief_nexus::GenerationId) {
        let Ok(entries) = fs::read_dir(&self.generations) else { return };
        let mut generations = entries.flatten().filter_map(|entry| {
            let path = entry.path();
            if !entry.file_type().ok()?.is_dir() { return None; }
            let manifest = read_manifest(&path.join("manifest.signal")).ok()?;
            Some((manifest.intended_publication.0.0, manifest.generation_id.0, path))
        }).collect::<Vec<_>>();
        generations.sort_by(|a, b| b.0.cmp(&a.0));
        let deletions = generations.into_iter().skip(retained).filter(|(_, id, _)| id != &current.0).map(|(_, _, path)| path).collect::<Vec<_>>();
        // `deletions` is the complete checked listing before any deletion runs.
        for path in deletions { let _ = fs::remove_dir_all(path); }
    }
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

fn relative_from(root: &Path, target: &Path) -> String {
    target.strip_prefix(root).unwrap_or(target).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use meta_signal_belief_nexus::{
        CurrentLink, GenerationsDirectory, InstructionsFile, InstructionsSetting, Layout,
        OwnedRoot, SettingsFile, SkillsDirectory,
    };
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

    fn publication(root: &Path, generation: &str, prior: Option<i64>, intended: i64, instruction: &str, skills: Vec<SkillFile>) -> Publication {
        Publication {
            workspace: Workspace(WorkspacePath(root.to_string_lossy().into_owned())),
            generation_id: GenerationId(generation.into()),
            prior_publication: PriorPublication(prior.map(PublicationRevision)),
            intended_publication: IntendedPublication(PublicationRevision(intended)),
            workspace_image: WorkspaceImage { layout: layout(), content: Content(instruction.into()), skill_file_vector: skills },
            repository_states: RepositoryStates(vec![RepositoryState {
                repository: Repository(RepositoryPath("/source".into())),
                source_revision: SourceRevision(format!("revision-{intended}")),
            }]),
            found_modules: FoundModules(vec![ModuleName { kind: Kind::Vision, subject: Subject("nexus".into()) }]),
            missing_modules: MissingModules(vec![],),
        }
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
            workspace_image: WorkspaceImage { layout: layout(), content: Content("instructions".into()), skill_file_vector: vec![] },
            repository_states: RepositoryStates(vec![]),
            found_modules: FoundModules(vec![]),
            missing_modules: MissingModules(vec![ModuleName { kind: Kind::Vision, subject: Subject("missing".into()) }]),
        };
        let query = Query::Publish(publication);
        let archived = query.signalize().expect("archive emission");
        assert_eq!(Signal::<Query>::from(archived.bytes().to_vec()).restore().unwrap(), query);
    }

    #[test]
    fn inspection_returns_pointer_and_manifest_as_separate_recovery_evidence() {
        let response = Response::Inspected(PublicationInspection {
            workspace: Workspace(WorkspacePath("/tmp/work".into())),
            owned_pointer_option: Some(OwnedPointer { generation_id: GenerationId("sha256:generation".into()), publication_revision: PublicationRevision(4) }),
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

        let emission = FileEmission::new(PathBuf::from("git"), 2).unwrap();
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
        let emission = FileEmission::new(PathBuf::from("git"), 2).unwrap();
        let first = publication(
            workspace.path(),
            "generation-one",
            None,
            1,
            "first instructions",
            vec![SkillFile { relative_path: RelativePath("nexus/SKILL.md".into()), content: Content("first skill".into()) }],
        );
        assert!(matches!(emission.execute(Query::Publish(first)), Response::Published(_)));
        assert_eq!(fs::read_link(workspace.path().join(".belief/current")).unwrap(), PathBuf::from("generations/generation-one"));
        assert_eq!(fs::read_link(workspace.path().join(".agents/skills")).unwrap(), PathBuf::from("../.belief/current/skills"));
        assert_eq!(fs::read_to_string(workspace.path().join("unrelated.txt")).unwrap(), "keep");
        let settings = fs::read_to_string(workspace.path().join(".codex/config.toml")).unwrap();
        assert!(settings.contains("model = \"gpt-test\""));
        assert!(settings.contains("mode = \"strict\""));
        assert!(settings.contains("model_instructions_file = \".belief/current/model-instructions.md\""));

        let invalid = publication(
            workspace.path(), "generation-invalid", Some(1), 2, "invalid",
            vec![SkillFile { relative_path: RelativePath("../escape".into()), content: Content("escape".into()) }],
        );
        assert!(matches!(emission.execute(Query::Publish(invalid)), Response::EmissionRefused(_)));
        assert_eq!(fs::read_link(workspace.path().join(".belief/current")).unwrap(), PathBuf::from("generations/generation-one"));
        assert!(!workspace.path().join(".belief/generations/generation-invalid").exists());

        let second = publication(workspace.path(), "generation-two", Some(1), 2, "second instructions", vec![]);
        assert!(matches!(emission.execute(Query::Publish(second)), Response::Published(_)));
        assert_eq!(fs::read_link(workspace.path().join(".belief/current")).unwrap(), PathBuf::from("generations/generation-two"));
        assert_eq!(fs::read_to_string(workspace.path().join(".belief/generations/generation-one/model-instructions.md")).unwrap(), "first instructions");

        let inspected = emission.execute(Query::InspectPublication(InspectionRequest {
            workspace: Workspace(WorkspacePath(workspace.path().to_string_lossy().into_owned())),
            layout: layout(),
        }));
        let Response::Inspected(observed) = inspected else { panic!("inspection refused: {inspected:?}") };
        assert_eq!(observed.owned_pointer_option.as_ref().unwrap().generation_id, GenerationId("generation-two".into()));
        let manifest = observed.publication_manifest_option.unwrap();
        assert_eq!(manifest.generation_id, GenerationId("generation-two".into()));
        assert_eq!(manifest.intended_publication, IntendedPublication(PublicationRevision(2)));
        assert_eq!(manifest.repository_states.0.len(), 1);
        assert_eq!(manifest.found_modules.0.len(), 1);
    }

    #[test]
    fn publication_refuses_unowned_skill_destination_and_owned_root_symlink() {
        let emission = FileEmission::new(PathBuf::from("git"), 2).unwrap();
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
}
