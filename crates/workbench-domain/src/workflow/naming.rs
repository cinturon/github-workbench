use crate::error::WorkbenchError;

/// Lowercase, hyphenated slug safe for Git ref path segments.
pub fn normalize_slug(title: &str) -> Result<String, WorkbenchError> {
    let mut out = String::new();
    let mut prev_hyphen = false;
    for ch in title.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_hyphen = false;
        } else if !prev_hyphen && !out.is_empty() {
            out.push('-');
            prev_hyphen = true;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        return Err(WorkbenchError::InvalidBranchName {
            reason: "slug is empty after normalization".into(),
        });
    }
    if out.contains("..") || out.starts_with('.') || out.ends_with('.') {
        return Err(WorkbenchError::InvalidBranchName {
            reason: "slug contains prohibited '.' sequences".into(),
        });
    }
    Ok(out)
}

pub fn branch_name(pattern: &str, issue: u64, title: &str) -> Result<String, WorkbenchError> {
    let slug = normalize_slug(title)?;
    let name = pattern
        .replace("{issue}", &issue.to_string())
        .replace("{slug}", &slug);
    validate_branch_ref(&name)?;
    Ok(name)
}

fn is_forbidden_ref_char(c: char) -> bool {
    // Git disallows ASCII control chars (incl. NUL, DEL) and these punctuation chars in refs.
    c.is_ascii_control() || matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
}

fn validate_branch_ref(name: &str) -> Result<(), WorkbenchError> {
    if name.is_empty()
        || name.contains("//")
        || name.contains("..")
        || name.starts_with('/')
        || name.ends_with('/')
        || name.ends_with(".lock")
        || name.contains("@{")
        || name.chars().any(is_forbidden_ref_char)
    {
        return Err(WorkbenchError::InvalidBranchName {
            reason: format!("prohibited ref characters in `{name}`"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn slug_never_contains_prohibited_ref_chars(title in ".{0,80}") {
            if let Ok(slug) = normalize_slug(&title) {
                prop_assert!(!slug.contains(".."));
                prop_assert!(!slug.contains(' '));
                prop_assert!(!slug.contains('@'));
                prop_assert!(!slug.contains('\\'));
                prop_assert!(!slug.is_empty());
                prop_assert_eq!(normalize_slug(&slug)?, slug);
            }
        }
    }
}
