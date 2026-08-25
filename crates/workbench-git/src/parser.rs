use workbench_application::AppError;
use workbench_domain::repository::Remote;

pub fn parse_porcelain_z(stdout: &str) -> Vec<String> {
    stdout
        .split('\0')
        .filter_map(|entry| entry.get(3..))
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn parse_remotes_verbose(stdout: &str) -> Vec<Remote> {
    let mut remotes = Vec::<(Remote, bool)>::new();

    for line in stdout.lines() {
        let Some((name, value)) = line.split_once('\t') else {
            continue;
        };
        let Some((url, kind)) = value.rsplit_once(' ') else {
            continue;
        };
        let is_fetch = kind == "(fetch)";
        if !is_fetch && kind != "(push)" {
            continue;
        }

        if let Some((remote, has_fetch_url)) =
            remotes.iter_mut().find(|(remote, _)| remote.name == name)
        {
            if is_fetch && !*has_fetch_url {
                remote.url = url.to_string();
                *has_fetch_url = true;
            }
        } else {
            remotes.push((
                Remote {
                    name: name.to_string(),
                    url: url.to_string(),
                },
                is_fetch,
            ));
        }
    }

    remotes.into_iter().map(|(remote, _)| remote).collect()
}

pub fn parse_ahead_behind(stdout: &str) -> Result<(u64, u64), AppError> {
    let mut fields = stdout.split_whitespace();
    let ahead = fields.next().and_then(|value| value.parse().ok());
    let behind = fields.next().and_then(|value| value.parse().ok());

    match (ahead, behind, fields.next()) {
        (Some(ahead), Some(behind), None) => Ok((ahead, behind)),
        _ => Err(AppError::Usage {
            message: format!("unexpected ahead/behind output from git: {stdout:?}"),
        }),
    }
}
