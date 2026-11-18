//! What the translator does in the world when the Nexus asks: one emission
//! attempt, done under the translator settings the attempt carries.

use crate::configuration::TranslatorConfiguration;
use crate::generated::ConfigurationFault;
use emission_belief_nexus as emission;
use emission_belief_nexus::FileEmission;
use nexus_exchange::{LaneFailure, LaneFault};

/// One emission attempt, answered.
pub trait Attempting {
    /// Does the attempt under the settings it carries and answers its
    /// outcome at the attempt's revision; an attempt whose settings are not
    /// coherent is declined before anything is done.
    fn attempted(self) -> emission::EmissionAnswer;
}

impl Attempting for emission::EmissionAttempt {
    fn attempted(self) -> emission::EmissionAnswer {
        match self.outcome() {
            Ok(outcome) => emission::EmissionAnswer::Attempted(outcome),
            Err(fault) => {
                emission::EmissionAnswer::LaneFailed(LaneFailure::Declined(LaneFault(fault.0)))
            }
        }
    }
}

trait Executing {
    fn outcome(self) -> Result<emission::EmissionAttemptOutcome, ConfigurationFault>;
}

impl Executing for emission::EmissionAttempt {
    fn outcome(self) -> Result<emission::EmissionAttemptOutcome, ConfigurationFault> {
        let configuration = TranslatorConfiguration::from_snapshot(&self.translator_configuration)?;
        if configuration.revision() != Some(&self.configuration_revision) {
            return Err(ConfigurationFault(
                "emission attempt revision does not match its snapshot".into(),
            ));
        }
        let git = configuration.git_executable().ok_or_else(|| {
            ConfigurationFault("emission attempt is awaiting Git executable".into())
        })?;
        let executor =
            FileEmission::new(std::path::PathBuf::from(git)).map_err(ConfigurationFault)?;
        Ok(emission::EmissionAttemptOutcome {
            configuration_revision: self.configuration_revision,
            output: executor.execute(self.input),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::snapshot_for_test;
    use meta_signal_belief_nexus as meta;
    use signal_belief_nexus as public;

    fn missing_repository() -> emission::Input {
        emission::Input::Snapshot(public::Repository(public::RepositoryPath(
            "/definitely/missing".into(),
        )))
    }

    #[test]
    fn emission_outcome_carries_the_attempt_revision() {
        let attempt = emission::EmissionAttempt {
            configuration_revision: meta::ConfigurationRevision(7),
            translator_configuration: snapshot_for_test(7),
            input: missing_repository(),
        };
        let emission::EmissionAnswer::Attempted(outcome) = attempt.attempted() else {
            panic!("a coherent attempt is attempted");
        };
        assert_eq!(outcome.configuration_revision, meta::ConfigurationRevision(7));
        assert!(matches!(outcome.output, emission::Output::EmissionRefused(_)));
    }

    #[test]
    fn an_attempt_whose_settings_are_not_its_revision_is_declined() {
        let attempt = emission::EmissionAttempt {
            configuration_revision: meta::ConfigurationRevision(8),
            translator_configuration: snapshot_for_test(7),
            input: missing_repository(),
        };
        assert!(matches!(
            attempt.attempted(),
            emission::EmissionAnswer::LaneFailed(LaneFailure::Declined(_))
        ));
    }
}
