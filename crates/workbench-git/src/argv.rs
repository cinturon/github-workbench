use workbench_domain::operations::plan::GitCommand;

pub fn command_argv(cmd: &GitCommand) -> Vec<String> {
    let args = match cmd {
        GitCommand::Fetch { remote } => vec!["fetch".into(), "--".into(), remote.clone()],
        GitCommand::CreateBranch { name, start_point } => vec![
            "checkout".into(),
            "-b".into(),
            name.clone(),
            "--".into(),
            start_point.clone(),
        ],
        GitCommand::PushRef {
            remote,
            local_ref,
            remote_ref,
            set_upstream,
        } => {
            let mut args = vec!["push".into()];
            if *set_upstream {
                args.push("-u".into());
            }
            args.extend([
                "--".into(),
                remote.clone(),
                format!("{local_ref}:{remote_ref}"),
            ]);
            args
        }
    };
    assert_no_force(&args);
    args
}

pub fn describe_command(cmd: &GitCommand) -> String {
    let mut parts = vec!["git".to_string()];
    parts.extend(command_argv(cmd));
    parts.join(" ")
}

pub fn assert_no_force(args: &[String]) {
    let forbidden = args.iter().any(|a| {
        a == "--force"
            || a == "--force-with-lease"
            || a.starts_with("--force=")
            || a.starts_with("--force-with-lease=")
    });
    assert!(
        !forbidden,
        "force push arguments are forbidden in Phase 2: {args:?}"
    );
}

#[cfg(test)]
mod tests {
    use workbench_domain::operations::plan::GitCommand;

    use crate::argv::command_argv;

    #[test]
    fn push_argv_never_contains_force() {
        let cmd = GitCommand::PushRef {
            remote: "github".into(),
            local_ref: "feature/x".into(),
            remote_ref: "feature/x".into(),
            set_upstream: true,
        };
        let args = command_argv(&cmd);
        assert!(args.iter().any(|a| a == "-u"));
        assert_eq!(args.last().map(String::as_str), Some("feature/x:feature/x"));
        assert!(!args.iter().any(|a| {
            a == "--force"
                || a == "--force-with-lease"
                || a.starts_with("--force=")
                || a.starts_with("--force-with-lease=")
        }));
    }

    #[test]
    fn create_branch_uses_checkout_b_and_dashdash() {
        let args = command_argv(&GitCommand::CreateBranch {
            name: "feature/42-add-resumable-uploads".into(),
            start_point: "main".into(),
        });
        assert_eq!(
            args,
            vec![
                "checkout",
                "-b",
                "feature/42-add-resumable-uploads",
                "--",
                "main"
            ]
        );
    }

    #[test]
    fn fetch_uses_dashdash() {
        assert_eq!(
            command_argv(&GitCommand::Fetch {
                remote: "github".into()
            }),
            vec!["fetch", "--", "github"]
        );
    }
}
