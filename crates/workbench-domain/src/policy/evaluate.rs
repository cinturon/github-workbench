use super::{Enforcement, PolicyConfig, PolicyFinding, Severity};

pub fn evaluate_commit_message_policy(policy: &PolicyConfig, message: &str) -> Vec<PolicyFinding> {
    let severity = match policy.commits.conventional_commits {
        Enforcement::Off => return Vec::new(),
        Enforcement::Warning => Severity::Warning,
        Enforcement::Blocker => Severity::Blocker,
    };

    if looks_like_conventional_commit(message) {
        return Vec::new();
    }

    vec![PolicyFinding {
        rule_id: "commits.conventional-commits".into(),
        severity,
        expected: "<type>[optional scope][!]: <description>".into(),
        actual: message.into(),
        message: "Commit message does not follow Conventional Commits syntax.".into(),
        remediation: "Use a message such as `feat: add policy evaluation`.".into(),
    }]
}

fn looks_like_conventional_commit(message: &str) -> bool {
    let Some((prefix, description)) = message
        .lines()
        .next()
        .and_then(|line| line.split_once(": "))
    else {
        return false;
    };
    if description.trim().is_empty() {
        return false;
    }

    let prefix = prefix.strip_suffix('!').unwrap_or(prefix);
    let (commit_type, scope) = match prefix.split_once('(') {
        Some((commit_type, scope)) => {
            let Some(scope) = scope.strip_suffix(')') else {
                return false;
            };
            (commit_type, Some(scope))
        }
        None => (prefix, None),
    };

    valid_token(commit_type)
        && match scope {
            Some(scope) => valid_scope(scope),
            None => true,
        }
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
}

fn valid_scope(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'/'))
}
