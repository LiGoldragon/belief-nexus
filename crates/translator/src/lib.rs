//! The Belief Nexus's translator: its own settings, followed from the
//! Nexus's answers, and the external effects it does for the Nexus; the
//! bodies it writes for the serving every translator shares
//! (`translator-serving`); and its CLIs on the client end
//! (`translator-client`).
//!
//! The kinds declared in `ethos/translator.ethos` are generated into
//! `src/generated/translator.rs` by ethos-zero; the flake's check refuses a
//! committed generation that differs from what the source generates.

pub mod configuration;
pub mod emission;
#[rustfmt::skip]
#[path = "generated/translator.rs"]
pub mod generated;
pub mod server;

pub use configuration::TranslatorConfiguration;
pub use server::{BeliefReady, BeliefTranslator};

/// The settings a place starts with, as its deployment names them: one
/// datom, a vector of the meta contract's `Set`, read whole within Signal's
/// own bounds.
pub fn deployment_sets(text: &str) -> Result<Vec<meta_signal_belief_nexus::Set>, String> {
    use translator_serving::Budgeting;
    shared_translator::decode::<Vec<meta_signal_belief_nexus::Set>>(
        text,
        &mut translator_serving::ValidTextBudget::MAXIMUM.budget(),
    )
    .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use meta_signal_belief_nexus as meta;
    use signal_belief_nexus as public;

    #[test]
    fn the_deployment_names_twenty_settings_each_once() {
        let sets = deployment_sets(include_str!("../deployment.datom")).expect("the deployment reads");
        assert_eq!(sets.len(), 20, "{sets:?}");
        assert_eq!(sets[0], meta::Set::RepositoryPolling(meta::Timeout(100)));
        assert!(sets.contains(&meta::Set::PlacementDefault(meta::PlacementDefault {
            kind: public::Kind::Vision,
            placement: public::Placement::Skills,
        })));
        assert!(sets.iter().any(|set| matches!(
            set,
            meta::Set::HarnessLayout(meta::HarnessLayout { harness: public::Harness::Claude, layout })
                if layout.instructions_setting.0 == "unused-for-claude"
                    && layout.skills_directory.0 .0 == ".claude/skills"
        )));
        assert_eq!(
            sets.last(),
            Some(&meta::Set::GitExecutable(public::ExecutablePath("@git@".into())))
        );
    }

    #[test]
    fn a_deployment_that_is_not_a_vector_of_settings_is_refused() {
        assert!(deployment_sets("RepositoryPolling.100").is_err());
        assert!(deployment_sets("[ NotASetting.1 ]").is_err());
    }
}
