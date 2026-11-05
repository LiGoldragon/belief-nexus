#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    clippy::type_complexity
)]
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Configuration {
    pub configuration_revision: meta_signal_belief_nexus::ConfigurationRevision,
    pub first_timeout_option: std::option::Option<meta_signal_belief_nexus::Timeout>,
    pub integer_option: std::option::Option<i64>,
    pub first_peer_list_option: std::option::Option<signal_belief_nexus::PeerList>,
    pub second_peer_list_option: std::option::Option<signal_belief_nexus::PeerList>,
    pub harness_layout_vector: std::vec::Vec<meta_signal_belief_nexus::HarnessLayout>,
    pub placement_default_vector: std::vec::Vec<
        meta_signal_belief_nexus::PlacementDefault,
    >,
    pub retention_policy: meta_signal_belief_nexus::RetentionPolicy,
    pub byte_limit_option: std::option::Option<meta_signal_belief_nexus::ByteLimit>,
    pub second_timeout_option: std::option::Option<meta_signal_belief_nexus::Timeout>,
    pub set_vector: std::vec::Vec<meta_signal_belief_nexus::Set>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct StoredRepository {
    pub owner: signal_belief_nexus::Owner,
    pub source_snapshot: emission_belief_nexus::SourceSnapshot,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct StoredWorkspace {
    pub owner: signal_belief_nexus::Owner,
    pub furnish_request: signal_belief_nexus::FurnishRequest,
    pub configuration_revision: meta_signal_belief_nexus::ConfigurationRevision,
    pub layout: meta_signal_belief_nexus::Layout,
    pub placement_default_vector: std::vec::Vec<
        meta_signal_belief_nexus::PlacementDefault,
    >,
    pub retention_policy: meta_signal_belief_nexus::RetentionPolicy,
    pub furnished_workspace_option: std::option::Option<
        signal_belief_nexus::FurnishedWorkspace,
    >,
    pub successful_generation_vector: std::vec::Vec<SuccessfulGeneration>,
    pub repository_state_vector: std::vec::Vec<signal_belief_nexus::RepositoryState>,
    pub publication_option: std::option::Option<emission_belief_nexus::Publication>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct SuccessfulGeneration {
    pub generation_id: signal_belief_nexus::GenerationId,
    pub publication_revision: signal_belief_nexus::PublicationRevision,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct CleanupMaintenance {
    pub workspace: signal_belief_nexus::Workspace,
    pub generation_id: signal_belief_nexus::GenerationId,
    pub publication_revision: signal_belief_nexus::PublicationRevision,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct MemoryImage {
    pub owner: signal_belief_nexus::Owner,
    pub configuration: Configuration,
    pub stored_repository_vector: std::vec::Vec<StoredRepository>,
    pub stored_workspace_vector: std::vec::Vec<StoredWorkspace>,
    pub cleanup_maintenance_vector: std::vec::Vec<CleanupMaintenance>,
    pub integer: i64,
}
