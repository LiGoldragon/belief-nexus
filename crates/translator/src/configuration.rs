//! The translator's own settings, kept by the Nexus beside its own under one
//! coherent configuration revision, and followed here from each answer the
//! Nexus gives.
//!
//! Hand-written: the settings are the meta contract's types held as a value
//! the translator changes; the fault is declared in `ethos/translator.ethos`.

use crate::generated::ConfigurationFault;
use meta_signal_belief_nexus as meta;
use signal_belief_nexus as public;
use translator_serving::ValidTextBudget;

fn fault(message: &str) -> ConfigurationFault {
    ConfigurationFault(message.into())
}

/// The translator's settings as the Nexus last answered them, at the
/// configuration revision they stand at.
#[derive(Clone, Debug, Default)]
pub struct TranslatorConfiguration {
    revision: Option<meta::ConfigurationRevision>,
    nexus_socket: Option<meta::SocketPath>,
    meta_nexus_socket: Option<meta::SocketPath>,
    call_timeout: Option<meta::Timeout>,
    output_limit: Option<meta::ByteLimit>,
    text_budget: Option<meta::Budget>,
    frame_capacity: Option<meta::ByteLimit>,
    transport_timeout: Option<meta::Timeout>,
    allowed_nexus_peers: Option<public::PeerList>,
    git_executable: Option<public::ExecutablePath>,
    snapshot_ready: bool,
}

fn unkeyed_addresses(names: Vec<meta::SettingName>) -> Vec<meta::SettingAddress> {
    names
        .into_iter()
        .map(|setting_name| meta::SettingAddress {
            setting_name,
            setting_key_option: None,
        })
        .collect()
}

fn translator_set_missing(sets: &[meta::Set]) -> Vec<meta::SettingName> {
    let required = [
        meta::SettingName::NexusSocket,
        meta::SettingName::MetaNexusSocket,
        meta::SettingName::CallTimeout,
        meta::SettingName::OutputLimit,
        meta::SettingName::TextBudget,
        meta::SettingName::TranslatorFrameCapacity,
        meta::SettingName::TranslatorTransportTimeout,
        meta::SettingName::AllowedNexusPeers,
        meta::SettingName::GitExecutable,
    ];
    required
        .into_iter()
        .filter(|name| !sets.iter().any(|set| setting_name_for_translator(set) == *name))
        .collect()
}

fn setting_name_for_translator(set: &meta::Set) -> meta::SettingName {
    match set {
        meta::Set::NexusSocket(_) => meta::SettingName::NexusSocket,
        meta::Set::MetaNexusSocket(_) => meta::SettingName::MetaNexusSocket,
        meta::Set::CallTimeout(_) => meta::SettingName::CallTimeout,
        meta::Set::OutputLimit(_) => meta::SettingName::OutputLimit,
        meta::Set::TextBudget(_) => meta::SettingName::TextBudget,
        meta::Set::TranslatorFrameCapacity(_) => meta::SettingName::TranslatorFrameCapacity,
        meta::Set::TranslatorTransportTimeout(_) => meta::SettingName::TranslatorTransportTimeout,
        meta::Set::AllowedNexusPeers(_) => meta::SettingName::AllowedNexusPeers,
        meta::Set::GitExecutable(_) => meta::SettingName::GitExecutable,
        _ => meta::SettingName::RepositoryPolling,
    }
}

/// Which of the two keeps a setting: the translator or the Nexus.
pub fn keeper(setting: &meta::Set) -> meta::Keeper {
    match setting {
        meta::Set::NexusSocket(_)
        | meta::Set::MetaNexusSocket(_)
        | meta::Set::CallTimeout(_)
        | meta::Set::OutputLimit(_)
        | meta::Set::TextBudget(_)
        | meta::Set::TranslatorFrameCapacity(_)
        | meta::Set::TranslatorTransportTimeout(_)
        | meta::Set::AllowedNexusPeers(_)
        | meta::Set::GitExecutable(_) => meta::Keeper::Translator,
        _ => meta::Keeper::Nexus,
    }
}

impl TranslatorConfiguration {
    /// The settings a snapshot names, checked whole: a translator setting
    /// each, once, at a revision that is not negative, ready only as its
    /// settings make it.
    pub fn from_snapshot(snapshot: &meta::TranslatorConfiguration) -> Result<Self, ConfigurationFault> {
        if snapshot.configuration_revision.0 < 0 {
            return Err(fault("negative configuration revision"));
        }
        let mut configuration = Self::default();
        let mut seen = std::collections::HashSet::new();
        for set in &snapshot.set_vector {
            if keeper(set) != meta::Keeper::Translator {
                return Err(fault("Nexus-owned setting in translator snapshot"));
            }
            if !seen.insert(std::mem::discriminant(set)) {
                return Err(fault("duplicate translator setting"));
            }
            configuration.apply_set(set.clone())?;
        }
        configuration.revision = Some(snapshot.configuration_revision);
        configuration.snapshot_ready = matches!(snapshot.readiness, meta::Readiness::Ready);
        let derived = if configuration.missing().is_empty() {
            meta::Readiness::Ready
        } else {
            meta::Readiness::Awaiting(unkeyed_addresses(configuration.missing()))
        };
        if derived != snapshot.readiness {
            return Err(fault("translator readiness is not derived from its settings"));
        }
        Ok(configuration)
    }

    /// Takes the Nexus's snapshot of the translator's settings, at its
    /// revision or a later one; an older snapshot changes nothing.
    pub fn restore(&mut self, snapshot: &meta::TranslatorConfiguration) -> Result<(), ConfigurationFault> {
        if snapshot.configuration_revision.0 < 0 {
            return Err(fault("negative configuration revision"));
        }
        if self
            .revision
            .as_ref()
            .is_some_and(|current| snapshot.configuration_revision.0 < current.0)
        {
            return Err(fault("stale configuration snapshot"));
        }
        let mut seen = std::collections::HashSet::new();
        let mut candidate = self.clone();
        for set in &snapshot.set_vector {
            if keeper(set) != meta::Keeper::Translator || !seen.insert(std::mem::discriminant(set)) {
                return Err(fault("invalid translator configuration snapshot"));
            }
            candidate.apply_set(set.clone())?;
        }
        candidate.revision = Some(snapshot.configuration_revision);
        let stored_missing = translator_set_missing(&snapshot.set_vector);
        let derived = if stored_missing.is_empty() {
            meta::Readiness::Ready
        } else {
            meta::Readiness::Awaiting(unkeyed_addresses(stored_missing))
        };
        if derived != snapshot.readiness {
            return Err(fault("translator readiness is not derived from its stored settings"));
        }
        candidate.snapshot_ready = matches!(snapshot.readiness, meta::Readiness::Ready);
        *self = candidate;
        Ok(())
    }

    pub fn revision(&self) -> Option<&meta::ConfigurationRevision> {
        self.revision.as_ref()
    }

    /// Follows one Configured answer of the Nexus. The next revision is
    /// applied; a revision the settings already stand at or past is already
    /// reflected and changes nothing; a skipped revision, a keeper that does
    /// not keep its setting or an exhausted revision is refused and leaves
    /// the settings whole.
    pub fn apply(&mut self, configured: &meta::ConfiguredState) -> Result<(), ConfigurationFault> {
        if configured.keeper != keeper(&configured.set) {
            return Err(fault("configured keeper does not own its setting"));
        }
        let expected = match self.revision.as_ref() {
            Some(revision) => revision
                .0
                .checked_add(1)
                .ok_or_else(|| fault("non-coherent configuration revision"))?,
            None => 0,
        };
        if configured.configuration_revision.0 < expected {
            return Ok(());
        }
        if configured.configuration_revision.0 != expected {
            return Err(fault("non-coherent configuration revision"));
        }
        let mut candidate = self.clone();
        if configured.keeper == meta::Keeper::Translator {
            candidate.apply_set(configured.set.clone())?;
        }
        candidate.revision = Some(configured.configuration_revision);
        candidate.snapshot_ready = matches!(configured.readiness, meta::Readiness::Ready);
        *self = candidate;
        Ok(())
    }

    fn apply_set(&mut self, setting: meta::Set) -> Result<(), ConfigurationFault> {
        match setting {
            meta::Set::NexusSocket(value) => self.nexus_socket = Some(value),
            meta::Set::MetaNexusSocket(value) => self.meta_nexus_socket = Some(value),
            meta::Set::CallTimeout(value) if value.0 > 0 => self.call_timeout = Some(value),
            meta::Set::OutputLimit(value) if value.0 > 0 => self.output_limit = Some(value),
            meta::Set::TextBudget(value)
                if ValidTextBudget::try_from(&translator_serving::TextLimits {
                    node_limit: translator_serving::NodeLimit(value.node_limit.0),
                    depth_limit: translator_serving::DepthLimit(value.depth_limit.0),
                })
                .is_ok() =>
            {
                self.text_budget = Some(value)
            }
            meta::Set::TranslatorFrameCapacity(value)
                if usize::try_from(value.0)
                    .ok()
                    .is_some_and(|capacity| capacity > 0 && capacity <= signal::MAXIMUM_SIGNAL_BYTES) =>
            {
                self.frame_capacity = Some(value)
            }
            meta::Set::TranslatorTransportTimeout(value) if value.0 > 0 => {
                self.transport_timeout = Some(value)
            }
            meta::Set::AllowedNexusPeers(value) if !value.0.is_empty() => {
                self.allowed_nexus_peers = Some(value)
            }
            meta::Set::GitExecutable(value) if !value.0.is_empty() => {
                self.git_executable = Some(value)
            }
            _ => return Err(fault("invalid translator-owned setting")),
        }
        Ok(())
    }

    fn missing(&self) -> Vec<meta::SettingName> {
        let mut missing = Vec::new();
        if self.nexus_socket.is_none() {
            missing.push(meta::SettingName::NexusSocket);
        }
        if self.meta_nexus_socket.is_none() {
            missing.push(meta::SettingName::MetaNexusSocket);
        }
        if self.call_timeout.is_none() {
            missing.push(meta::SettingName::CallTimeout);
        }
        if self.output_limit.is_none() {
            missing.push(meta::SettingName::OutputLimit);
        }
        if self.text_budget.is_none() {
            missing.push(meta::SettingName::TextBudget);
        }
        if self.frame_capacity.is_none() {
            missing.push(meta::SettingName::TranslatorFrameCapacity);
        }
        if self.transport_timeout.is_none() {
            missing.push(meta::SettingName::TranslatorTransportTimeout);
        }
        if self.allowed_nexus_peers.is_none() {
            missing.push(meta::SettingName::AllowedNexusPeers);
        }
        if self.git_executable.is_none() {
            missing.push(meta::SettingName::GitExecutable);
        }
        missing
    }

    /// Every translator setting is set, and the Nexus last answered the
    /// configuration ready.
    pub fn ready(&self) -> bool {
        self.snapshot_ready && self.missing().is_empty()
    }

    pub fn ordinary_nexus_socket(&self) -> Option<&str> {
        self.nexus_socket.as_ref().map(|value| value.0.as_str())
    }

    /// The text budget ordinary text is read within, once it is set.
    pub fn text_budget(&self) -> Option<ValidTextBudget> {
        self.text_budget.as_ref().and_then(|value| {
            ValidTextBudget::try_from(&translator_serving::TextLimits {
                node_limit: translator_serving::NodeLimit(value.node_limit.0),
                depth_limit: translator_serving::DepthLimit(value.depth_limit.0),
            })
            .ok()
        })
    }

    pub fn git_executable(&self) -> Option<&str> {
        self.git_executable.as_ref().map(|value| value.0.as_str())
    }
}

#[cfg(test)]
pub(crate) use tests::snapshot_for_test;

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_snapshot(revision: i64, caller: &shared_translator::Caller) -> meta::TranslatorConfiguration {
        let peer = shared_translator::PeerBinding {
            peer_id: shared_translator::PeerId("nexus".into()),
            peer_identity: shared_translator::PeerIdentity {
                user_id: caller.user_id,
                process_id: caller.process_id,
            },
        };
        meta::TranslatorConfiguration {
            configuration_revision: meta::ConfigurationRevision(revision),
            set_vector: vec![
                meta::Set::NexusSocket(meta::SocketPath("ordinary".into())),
                meta::Set::MetaNexusSocket(meta::SocketPath("meta".into())),
                meta::Set::CallTimeout(meta::Timeout(100)),
                meta::Set::OutputLimit(meta::ByteLimit(1000)),
                meta::Set::TextBudget(meta::Budget {
                    node_limit: meta::NodeLimit(1000),
                    depth_limit: meta::DepthLimit(64),
                }),
                meta::Set::TranslatorFrameCapacity(meta::ByteLimit(1000)),
                meta::Set::TranslatorTransportTimeout(meta::Timeout(100)),
                meta::Set::AllowedNexusPeers(public::PeerList(vec![peer])),
                meta::Set::GitExecutable(public::ExecutablePath("git".into())),
            ],
            readiness: meta::Readiness::Ready,
        }
    }

    pub(crate) fn current_caller() -> shared_translator::Caller {
        shared_translator::Caller {
            user_id: shared_translator::UserId(i64::from(nix::unistd::getuid().as_raw())),
            process_id: shared_translator::ProcessId(i64::from(std::process::id())),
        }
    }

    fn assert_same_configuration(actual: &TranslatorConfiguration, expected: &TranslatorConfiguration) {
        assert_eq!(actual.revision, expected.revision);
        assert_eq!(actual.nexus_socket, expected.nexus_socket);
        assert_eq!(actual.meta_nexus_socket, expected.meta_nexus_socket);
        assert_eq!(actual.call_timeout, expected.call_timeout);
        assert_eq!(actual.output_limit, expected.output_limit);
        assert_eq!(actual.text_budget, expected.text_budget);
        assert_eq!(actual.frame_capacity, expected.frame_capacity);
        assert_eq!(actual.transport_timeout, expected.transport_timeout);
        assert_eq!(actual.allowed_nexus_peers, expected.allowed_nexus_peers);
        assert_eq!(actual.git_executable, expected.git_executable);
        assert_eq!(actual.snapshot_ready, expected.snapshot_ready);
    }

    #[test]
    fn configuration_rejects_revision_gaps_without_mutation() {
        let mut configuration = TranslatorConfiguration::default();
        let gap = meta::ConfiguredState {
            set: meta::Set::CallTimeout(meta::Timeout(100)),
            keeper: meta::Keeper::Translator,
            configuration_revision: meta::ConfigurationRevision(2),
            readiness: meta::Readiness::Awaiting(vec![]),
        };
        assert!(configuration.apply(&gap).is_err());
        assert!(configuration.revision().is_none());
        let first = meta::ConfiguredState {
            configuration_revision: meta::ConfigurationRevision(0),
            ..gap
        };
        configuration.apply(&first).unwrap();
        assert_eq!(configuration.revision(), Some(&meta::ConfigurationRevision(0)));
    }

    #[test]
    fn a_revision_already_reflected_changes_nothing() {
        let caller = current_caller();
        let initial = TranslatorConfiguration::from_snapshot(&complete_snapshot(8, &caller)).unwrap();
        let mut configuration = initial.clone();
        configuration
            .apply(&meta::ConfiguredState {
                set: meta::Set::NexusSocket(meta::SocketPath("earlier".into())),
                keeper: meta::Keeper::Translator,
                configuration_revision: meta::ConfigurationRevision(7),
                readiness: meta::Readiness::Ready,
            })
            .unwrap();
        assert_same_configuration(&configuration, &initial);
    }

    #[test]
    fn configuration_refuses_exhausted_revision_without_mutation() {
        let caller = current_caller();
        let mut configuration =
            TranslatorConfiguration::from_snapshot(&complete_snapshot(i64::MAX, &caller)).unwrap();
        let exhausted = meta::ConfiguredState {
            set: meta::Set::NexusSocket(meta::SocketPath("changed".into())),
            keeper: meta::Keeper::Translator,
            configuration_revision: meta::ConfigurationRevision(i64::MIN),
            readiness: meta::Readiness::Ready,
        };

        assert!(configuration.apply(&exhausted).is_err());
        assert_eq!(configuration.revision(), Some(&meta::ConfigurationRevision(i64::MAX)));
        assert_eq!(configuration.ordinary_nexus_socket(), Some("ordinary"));
        assert!(configuration.ready());
    }

    #[test]
    fn configured_keeper_must_own_its_setting_before_any_state_changes() {
        let caller = current_caller();
        let initial = TranslatorConfiguration::from_snapshot(&complete_snapshot(7, &caller)).unwrap();
        for mismatched in [
            meta::ConfiguredState {
                set: meta::Set::NexusSocket(meta::SocketPath("changed".into())),
                keeper: meta::Keeper::Nexus,
                configuration_revision: meta::ConfigurationRevision(8),
                readiness: meta::Readiness::Awaiting(vec![]),
            },
            meta::ConfiguredState {
                set: meta::Set::RepositoryPolling(meta::Timeout(250)),
                keeper: meta::Keeper::Translator,
                configuration_revision: meta::ConfigurationRevision(8),
                readiness: meta::Readiness::Awaiting(vec![]),
            },
        ] {
            let mut configuration = initial.clone();
            assert!(configuration.apply(&mismatched).is_err());
            assert_same_configuration(&configuration, &initial);
        }

        let mut translator_owned = initial.clone();
        translator_owned
            .apply(&meta::ConfiguredState {
                set: meta::Set::NexusSocket(meta::SocketPath("changed".into())),
                keeper: meta::Keeper::Translator,
                configuration_revision: meta::ConfigurationRevision(8),
                readiness: meta::Readiness::Ready,
            })
            .unwrap();
        assert_eq!(translator_owned.revision(), Some(&meta::ConfigurationRevision(8)));
        assert_eq!(translator_owned.ordinary_nexus_socket(), Some("changed"));
        assert!(translator_owned.ready());

        let mut nexus_owned = initial;
        nexus_owned
            .apply(&meta::ConfiguredState {
                set: meta::Set::RepositoryPolling(meta::Timeout(250)),
                keeper: meta::Keeper::Nexus,
                configuration_revision: meta::ConfigurationRevision(8),
                readiness: meta::Readiness::Awaiting(unkeyed_addresses(vec![
                    meta::SettingName::RepositoryPolling,
                ])),
            })
            .unwrap();
        assert_eq!(nexus_owned.revision(), Some(&meta::ConfigurationRevision(8)));
        assert_eq!(nexus_owned.ordinary_nexus_socket(), Some("ordinary"));
        assert!(!nexus_owned.ready());
    }

    #[test]
    fn snapshots_refuse_negative_revisions_before_construction_or_restore_mutation() {
        let caller = current_caller();
        for invalid in [-1, i64::MIN] {
            let mut snapshot = complete_snapshot(invalid, &caller);
            snapshot.set_vector[0] = meta::Set::NexusSocket(meta::SocketPath("invalid".into()));
            assert!(TranslatorConfiguration::from_snapshot(&snapshot).is_err());

            let mut current = TranslatorConfiguration::from_snapshot(&complete_snapshot(8, &caller)).unwrap();
            let before = current.clone();
            assert!(current.restore(&snapshot).is_err());
            assert_same_configuration(&current, &before);
        }

        let zero = complete_snapshot(0, &caller);
        assert_eq!(
            TranslatorConfiguration::from_snapshot(&zero).unwrap().revision(),
            Some(&meta::ConfigurationRevision(0))
        );
        let mut restored = TranslatorConfiguration::default();
        restored.restore(&zero).unwrap();
        assert_eq!(restored.revision(), Some(&meta::ConfigurationRevision(0)));
        assert!(restored.ready());
    }

    #[test]
    fn restore_refuses_stale_snapshots_and_reconciles_equal_or_newer_revisions() {
        let caller = current_caller();
        let initial = TranslatorConfiguration::from_snapshot(&complete_snapshot(8, &caller)).unwrap();

        let mut stale = complete_snapshot(7, &caller);
        stale.set_vector[0] = meta::Set::NexusSocket(meta::SocketPath("stale".into()));
        let mut current = initial.clone();
        assert!(current.restore(&stale).is_err());
        assert_same_configuration(&current, &initial);

        let mut equal = complete_snapshot(8, &caller);
        equal.set_vector[0] = meta::Set::NexusSocket(meta::SocketPath("equal".into()));
        current.restore(&equal).unwrap();
        assert_eq!(current.revision(), Some(&meta::ConfigurationRevision(8)));
        assert_eq!(current.ordinary_nexus_socket(), Some("equal"));
        assert!(current.ready());

        let mut newer = complete_snapshot(10, &caller);
        newer.set_vector[0] = meta::Set::NexusSocket(meta::SocketPath("newer".into()));
        current.restore(&newer).unwrap();
        assert_eq!(current.revision(), Some(&meta::ConfigurationRevision(10)));
        assert_eq!(current.ordinary_nexus_socket(), Some("newer"));
        assert!(current.ready());
    }

    #[test]
    fn snapshot_rejects_duplicate_nexus_owned_and_false_readiness() {
        let caller = current_caller();
        let mut duplicate = complete_snapshot(1, &caller);
        duplicate.set_vector.push(meta::Set::CallTimeout(meta::Timeout(200)));
        assert!(TranslatorConfiguration::from_snapshot(&duplicate).is_err());

        let mut nexus_owned = complete_snapshot(1, &caller);
        nexus_owned.set_vector.push(meta::Set::RepositoryPolling(meta::Timeout(10)));
        assert!(TranslatorConfiguration::from_snapshot(&nexus_owned).is_err());

        let mut false_readiness = complete_snapshot(1, &caller);
        false_readiness.set_vector.pop();
        assert!(TranslatorConfiguration::from_snapshot(&false_readiness).is_err());
    }

    pub(crate) fn snapshot_for_test(revision: i64) -> meta::TranslatorConfiguration {
        complete_snapshot(revision, &current_caller())
    }
}
