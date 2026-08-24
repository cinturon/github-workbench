use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryId {
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remote {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchState {
    pub name: String,
    pub head_oid: Option<String>,
    pub upstream: Option<String>,
    pub base_branch: Option<String>,
    pub ahead: u64,
    pub behind: u64,
    pub dirty_paths: Vec<String>,
    pub is_protected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteIdentity {
    pub host: String,
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositorySnapshot {
    pub root: String,
    pub branch: Option<String>,
    pub detached_head: bool,
    pub head_oid: Option<String>,
    pub dirty_paths: Vec<String>,
    pub remotes: Vec<Remote>,
    pub selected_remote: Option<String>,
    pub upstream: Option<String>,
}

pub fn parse_github_remote(url: &str) -> Option<RemoteIdentity> {
    let url = url.trim();
    let url = url.strip_suffix(".git").unwrap_or(url);

    if let Some(rest) = url.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        return identity_from_host_path(host, path);
    }
    if let Some(rest) = url.strip_prefix("ssh://") {
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        let (host, path) = rest.split_once('/')?;
        let host = host.split_once('@').map(|(_, h)| h).unwrap_or(host);
        return identity_from_host_path(host, path);
    }
    if let Some(rest) = url.strip_prefix("https://") {
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        let (host, path) = rest.split_once('/')?;
        return identity_from_host_path(host, path);
    }
    if let Some(rest) = url.strip_prefix("http://") {
        let (host, path) = rest.split_once('/')?;
        return identity_from_host_path(host, path);
    }
    None
}

fn identity_from_host_path(host: &str, path: &str) -> Option<RemoteIdentity> {
    let path = path.trim_start_matches('/');
    let (owner, name) = path.split_once('/')?;
    if owner.is_empty() || name.is_empty() || name.contains('/') {
        return None;
    }
    Some(RemoteIdentity {
        host: host.to_string(),
        owner: owner.to_string(),
        name: name.to_string(),
    })
}
