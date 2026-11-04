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

#[cfg(test)]
mod tests {
    use super::*;
    use meta_signal_belief_nexus::{
        CurrentLink, GenerationsDirectory, InstructionsFile, InstructionsSetting, Layout,
        OwnedRoot, SettingsFile, SkillsDirectory,
    };
    use signal::{ByteViewable, Restorable, Signal, Signalizable};
    use signal_belief_nexus::{
        GenerationId, Kind, ModuleName, PublicationRevision, RelativePath, Subject,
        Workspace, WorkspacePath,
    };

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
}
