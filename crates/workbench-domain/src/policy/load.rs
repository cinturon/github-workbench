use super::{PolicyConfig, PolicyFinding, Severity};
use crate::WorkbenchError;

pub fn parse_policy_yaml(yaml: &str) -> Result<PolicyConfig, WorkbenchError> {
    let config = serde_yaml::from_str::<PolicyConfig>(yaml).map_err(|error| {
        let detail = error.to_string();
        let (rule_id, message, remediation) = if detail.contains("unknown field") {
            (
                "policy.unknown-field",
                "Policy contains an unknown field.",
                "Remove the unknown field or correct its kebab-case name.",
            )
        } else {
            (
                "policy.invalid-yaml",
                "Policy YAML does not match schema version 1.",
                "Correct the YAML value or add the required field.",
            )
        };

        invalid_policy(
            rule_id,
            "valid policy schema v1",
            detail,
            message,
            remediation,
        )
    })?;

    if config.schema_version != 1 {
        return Err(invalid_policy(
            "policy.schema-version",
            "1",
            config.schema_version.to_string(),
            "Policy schema version is unsupported.",
            "Set schema-version to 1.",
        ));
    }

    if config.strategy.preset != "github-flow" {
        return Err(invalid_policy(
            "policy.strategy.preset",
            "github-flow",
            config.strategy.preset.clone(),
            "Policy strategy preset is unsupported.",
            "Set strategy.preset to github-flow.",
        ));
    }

    Ok(config)
}

pub fn merge_policy(mut base: PolicyConfig, overlay: PolicyConfig) -> PolicyConfig {
    base.schema_version = overlay.schema_version;
    base.strategy = overlay.strategy;
    base.branches = overlay.branches;
    base.commits = overlay.commits;
    base.pull_requests = overlay.pull_requests;
    base
}

fn invalid_policy(
    rule_id: &str,
    expected: impl Into<String>,
    actual: impl Into<String>,
    message: &str,
    remediation: &str,
) -> WorkbenchError {
    WorkbenchError::InvalidPolicy {
        findings: vec![PolicyFinding {
            rule_id: rule_id.into(),
            severity: Severity::Blocker,
            expected: expected.into(),
            actual: actual.into(),
            message: message.into(),
            remediation: remediation.into(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::github_flow_defaults;

    #[test]
    fn defaults_round_trip_yaml() {
        let cfg = github_flow_defaults();
        let yaml = serde_yaml::to_string(&cfg).unwrap();
        let parsed = parse_policy_yaml(&yaml).unwrap();
        assert_eq!(parsed.schema_version, cfg.schema_version);
        assert_eq!(parsed.strategy, cfg.strategy);
        assert_eq!(
            parsed.branches.feature.pattern,
            cfg.branches.feature.pattern
        );
    }
}
