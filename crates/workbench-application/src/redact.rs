const MAX_OUTPUT_CHARS: usize = 64 * 1024;

pub fn redact(text: &str) -> String {
    let mut out = redact_basic_auth(text);
    out = redact_token_prefix(&out, "ghp_");
    out = redact_token_prefix(&out, "gho_");
    out = redact_token_prefix(&out, "ghu_");
    out = redact_token_prefix(&out, "ghs_");
    out = redact_token_prefix(&out, "github_pat_");
    out = redact_bearer(&out);
    out
}

pub fn bound_output(text: &str) -> String {
    let redacted = redact(text);
    if redacted.chars().count() <= MAX_OUTPUT_CHARS {
        return redacted;
    }
    let truncated: String = redacted.chars().take(MAX_OUTPUT_CHARS).collect();
    format!("{truncated}\n...[truncated]")
}

fn redact_basic_auth(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut search_from = 0;
    while let Some(rel) = text[search_from..].find("://") {
        let cred_start = search_from + rel + 3;
        out.push_str(&text[last..cred_start]);
        if let Some(rel_at) = text[cred_start..].find('@') {
            let creds = &text[cred_start..cred_start + rel_at];
            if creds.contains(':') && !creds.contains('/') {
                out.push_str("[redacted]");
                last = cred_start + rel_at;
                search_from = last;
                continue;
            }
        }
        last = cred_start;
        search_from = cred_start;
    }
    out.push_str(&text[last..]);
    out
}

fn redact_token_prefix(text: &str, prefix: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find(prefix) {
        out.push_str(&rest[..idx]);
        out.push_str("[redacted]");
        rest = &rest[idx + prefix.len()..];
        let skip = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .count();
        rest = &rest[rest
            .char_indices()
            .nth(skip)
            .map(|(i, _)| i)
            .unwrap_or(rest.len())..];
    }
    out.push_str(rest);
    out
}

fn redact_bearer(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.to_ascii_lowercase().find("bearer ") {
        out.push_str(&rest[..idx]);
        out.push_str("Bearer [redacted]");
        rest = &rest[idx + 7..];
        let skip = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
            .count();
        rest = &rest[rest
            .char_indices()
            .nth(skip)
            .map(|(i, _)| i)
            .unwrap_or(rest.len())..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_github_pat_and_basic_auth() {
        let input = "https://x-access-token:ghp_abcdefghijklmnopqrstuvwxyz012345@github.com/acme/widgets.git\nAuthorization: Bearer gho_abcdefghijklmnopqrstuvwxyz012345";
        let out = redact(input);
        assert!(!out.contains("ghp_abcdefghijklmnopqrstuvwxyz012345"));
        assert!(!out.contains("gho_abcdefghijklmnopqrstuvwxyz012345"));
        assert!(out.contains("[redacted]"));
    }

    #[test]
    fn truncates_long_output() {
        let huge = "a".repeat(70_000);
        let out = bound_output(&huge);
        assert!(out.len() < 70_000);
        assert!(out.contains("[truncated]"));
    }
}
