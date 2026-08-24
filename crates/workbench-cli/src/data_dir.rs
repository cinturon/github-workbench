use std::path::PathBuf;

pub fn resolve_data_dir(vars: impl Fn(&str) -> Option<String>) -> PathBuf {
    if let Some(dir) = vars("GWW_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(xdg) = vars("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("github-workbench");
    }
    if let Some(local) = vars("LOCALAPPDATA") {
        return PathBuf::from(local).join("github-workbench");
    }
    let home = vars("HOME").unwrap_or_else(|| ".".into());
    PathBuf::from(home).join(".local/share/github-workbench")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn vars<'a>(pairs: &'a [(&str, &str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn gww_data_dir_wins() {
        let path = resolve_data_dir(vars(&[
            ("GWW_DATA_DIR", "/tmp/gww"),
            ("XDG_DATA_HOME", "/xdg"),
            ("HOME", "/home/dev"),
        ]));
        assert_eq!(path, PathBuf::from("/tmp/gww"));
    }

    #[test]
    fn xdg_data_home_used() {
        let path = resolve_data_dir(vars(&[("XDG_DATA_HOME", "/xdg"), ("HOME", "/home/dev")]));
        assert_eq!(path, PathBuf::from("/xdg/github-workbench"));
    }

    #[test]
    fn windows_localappdata_used() {
        let path = resolve_data_dir(vars(&[("LOCALAPPDATA", "C:\\Users\\dev\\AppData\\Local")]));
        assert_eq!(
            path,
            PathBuf::from("C:\\Users\\dev\\AppData\\Local").join("github-workbench")
        );
    }

    #[test]
    fn home_fallback() {
        let path = resolve_data_dir(vars(&[("HOME", "/home/dev")]));
        assert_eq!(path, PathBuf::from("/home/dev/.local/share/github-workbench"));
    }
}
