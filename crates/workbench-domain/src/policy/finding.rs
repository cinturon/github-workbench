use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Blocker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyFinding {
    pub rule_id: String,
    pub severity: Severity,
    pub expected: String,
    pub actual: String,
    pub message: String,
    pub remediation: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn blocker_finding_serializes_severity_lowercase() {
        let f = PolicyFinding {
            rule_id: "pull-requests.required-base".into(),
            severity: Severity::Blocker,
            expected: "main".into(),
            actual: "develop".into(),
            message: "Feature PRs must target main.".into(),
            remediation: "Change the pull request base to main.".into(),
        };
        let v = serde_yaml::to_value(&f).unwrap();
        assert_eq!(v["severity"].as_str(), Some("blocker"));
    }
}
