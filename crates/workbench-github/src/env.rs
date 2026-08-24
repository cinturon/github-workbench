const WHITELIST: &[&str] = &[
    "PATH",
    "HOME",
    "USERPROFILE",
    "XDG_CONFIG_HOME",
    "GH_HOST",
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "NO_COLOR",
];

pub fn sanitized_env() -> Vec<(String, String)> {
    WHITELIST
        .iter()
        .filter_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| ((*key).to_string(), value))
        })
        .collect()
}
