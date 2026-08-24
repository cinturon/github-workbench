use std::path::Path;

use crate::error::AppError;
use crate::ports::PolicySource;
use workbench_domain::policy::{github_flow_defaults, parse_policy_yaml, PolicyConfig};

pub struct FilePolicySource;

impl PolicySource for FilePolicySource {
    fn read_yaml(&self, repo_root: &Path) -> Result<Option<String>, AppError> {
        let path = repo_root.join(".github-workbench.yml");
        match std::fs::read_to_string(&path) {
            Ok(body) => Ok(Some(body)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(AppError::Io {
                path: path.display().to_string(),
                detail: err.to_string(),
            }),
        }
    }
}

pub fn load_policy<P: PolicySource>(
    source: &P,
    repo_root: &Path,
) -> Result<(PolicyConfig, &'static str), AppError> {
    match source.read_yaml(repo_root)? {
        None => Ok((github_flow_defaults(), "defaults")),
        Some(yaml) => {
            let cfg = parse_policy_yaml(&yaml)?;
            Ok((cfg, "file"))
        }
    }
}
