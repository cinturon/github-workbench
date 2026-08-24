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
