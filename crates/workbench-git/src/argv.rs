use workbench_domain::operations::plan::GitCommand;

pub fn command_argvs(cmd: &GitCommand) -> Vec<Vec<String>> {
    let commands = match cmd {
        GitCommand::Fetch { remote } => {
            vec![vec!["fetch".into(), "--".into(), remote.clone()]]
        }
        GitCommand::CreateBranch { name, start_point } => vec![vec![
            "checkout".into(),
            "-b".into(),
            name.clone(),
            start_point.clone(),
            "--".into(),
        ]],
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
            vec![args]
        }
        GitCommand::CommitPaths { message, paths } => {
            let mut add = vec!["add".into(), "--".into()];
            add.extend(paths.iter().cloned());
            let mut commit = vec!["commit".into(), "-m".into(), message.clone(), "--".into()];
            commit.extend(paths.iter().cloned());
            vec![add, commit]
        }
        GitCommand::DeleteRemoteRef { remote, ref_name } => vec![vec![
            "push".into(),
            "--".into(),
            remote.clone(),
            format!(":refs/heads/{ref_name}"),
        ]],
    };

    for args in &commands {
        assert_no_force(args);
    }
    commands
}

pub fn command_argv(cmd: &GitCommand) -> Vec<String> {
    let argvs = command_argvs(cmd);
    assert!(
        argvs.len() == 1,
        "command_argv expects a single git invocation: {cmd:?}"
    );
    argvs.into_iter().next().unwrap()
}

pub fn describe_command(cmd: &GitCommand) -> String {
    command_argvs(cmd)
        .into_iter()
        .map(|args| {
            let mut parts = vec!["git".to_string()];
            parts.extend(args);
            parts.join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
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

    use crate::argv::{command_argv, command_argvs};

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
                "main",
                "--"
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

    #[test]
    fn commit_paths_stages_and_commits_only_named_paths() {
        let commands = command_argvs(&GitCommand::CommitPaths {
            message: "chore: add remote action test".into(),
            paths: vec![".github/workflows/github-workbench-test-01JABC.yml".into()],
        });

        assert_eq!(
            commands,
            vec![
                vec![
                    "add",
                    "--",
                    ".github/workflows/github-workbench-test-01JABC.yml",
                ],
                vec![
                    "commit",
                    "-m",
                    "chore: add remote action test",
                    "--",
                    ".github/workflows/github-workbench-test-01JABC.yml",
                ],
            ]
        );
    }

    #[test]
    fn delete_ref_uses_a_non_force_empty_source_refspec() {
        let commands = command_argvs(&GitCommand::DeleteRemoteRef {
            remote: "github".into(),
            ref_name: "github-workbench/test/01JABC".into(),
        });

        assert_eq!(
            commands,
            vec![vec![
                "push",
                "--",
                "github",
                ":refs/heads/github-workbench/test/01JABC",
            ]]
        );
        assert!(!commands
            .iter()
            .flatten()
            .any(|argument| argument.contains("force")));
    }
}
