const WHITELIST: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "USERNAME",
    "USERPROFILE",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TERM",
    "TMPDIR",
    "TMP",
    "TEMP",
    "SSH_AUTH_SOCK",
    "SSH_AGENT_PID",
    "GIT_ASKPASS",
    "GIT_SSH",
    "GIT_EXEC_PATH",
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_SYSTEM",
    "GIT_CONFIG_NOSYSTEM",
    "GIT_AUTHOR_NAME",
    "GIT_AUTHOR_EMAIL",
    "GIT_COMMITTER_NAME",
    "GIT_COMMITTER_EMAIL",
];

pub fn sanitized_env(overrides: &[(String, String)]) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = WHITELIST
        .iter()
        .filter_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| ((*key).to_string(), value))
        })
        .collect();

    env.push(("GIT_TERMINAL_PROMPT".to_string(), "0".to_string()));

    for (key, value) in overrides {
        if key == "GWW_GIT_PROGRAM" {
            continue;
        }
        if let Some(existing) = env.iter_mut().find(|(k, _)| k == key) {
            existing.1.clone_from(value);
        } else {
            env.push((key.clone(), value.clone()));
        }
    }

    env
}
