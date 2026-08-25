use crate::error::AppError;
use workbench_domain::repository::Remote;

pub fn resolve_remote(
    remotes: &[Remote],
    mapped_remote: Option<&str>,
    flag: Option<&str>,
) -> Result<String, AppError> {
    if remotes.is_empty() {
        return Err(AppError::RepositoryNotMapped);
    }
    if let Some(name) = flag {
        return require_existing(remotes, name);
    }
    if let Some(name) = mapped_remote {
        return require_existing(remotes, name);
    }
    if remotes.len() == 1 {
        return Ok(remotes[0].name.clone());
    }
    Err(AppError::RemoteNotResolved {
        candidates: remotes.iter().map(|r| r.name.clone()).collect(),
    })
}

fn require_existing(remotes: &[Remote], name: &str) -> Result<String, AppError> {
    if remotes.iter().any(|r| r.name == name) {
        Ok(name.to_string())
    } else {
        Err(AppError::RemoteNotResolved {
            candidates: remotes.iter().map(|r| r.name.clone()).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remotes(names: &[&str]) -> Vec<Remote> {
        names
            .iter()
            .map(|name| Remote {
                name: (*name).into(),
                url: format!("git@github.com:acme/{name}.git"),
            })
            .collect()
    }

    #[test]
    fn flag_wins() {
        assert_eq!(
            resolve_remote(
                &remotes(&["origin", "github"]),
                Some("origin"),
                Some("github")
            )
            .unwrap(),
            "github"
        );
    }

    #[test]
    fn mapped_used_when_no_flag() {
        assert_eq!(
            resolve_remote(&remotes(&["origin", "github"]), Some("github"), None).unwrap(),
            "github"
        );
    }

    #[test]
    fn sole_remote_used() {
        assert_eq!(
            resolve_remote(&remotes(&["github"]), None, None).unwrap(),
            "github"
        );
    }

    #[test]
    fn multiple_unmapped_is_error() {
        let err = resolve_remote(&remotes(&["origin", "github"]), None, None).unwrap_err();
        assert!(matches!(err, AppError::RemoteNotResolved { .. }));
    }

    #[test]
    fn no_remotes_is_not_mapped() {
        let err = resolve_remote(&[], None, None).unwrap_err();
        assert!(matches!(err, AppError::RepositoryNotMapped));
    }
}
