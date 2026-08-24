use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "gww", version, about = "GitHub Workflow Workbench")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Open {
        path: PathBuf,
        #[arg(long)]
        remote: Option<String>,
    },
    Status {
        #[arg(long)]
        json: bool,
    },
    Issue {
        #[command(subcommand)]
        command: IssueCommands,
    },
    Push {
        #[arg(long)]
        plan: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        remote: Option<String>,
    },
    Ops {
        #[command(subcommand)]
        command: OpsCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum IssueCommands {
    Start {
        number: u64,
        #[arg(long)]
        title: String,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        remote: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum OpsCommands {
    List,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_issue_start() {
        let cli = Cli::try_parse_from([
            "gww",
            "issue",
            "start",
            "42",
            "--title",
            "Add resumable uploads",
            "--yes",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Issue {
                command:
                    IssueCommands::Start {
                        number, title, yes, ..
                    },
            }) => {
                assert_eq!(number, 42);
                assert_eq!(title, "Add resumable uploads");
                assert!(yes);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parses_push_plan() {
        let cli = Cli::try_parse_from(["gww", "push", "--plan"]).unwrap();
        match cli.command {
            Some(Commands::Push { plan, yes, .. }) => {
                assert!(plan);
                assert!(!yes);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parses_status_json() {
        let cli = Cli::try_parse_from(["gww", "status", "--json"]).unwrap();
        match cli.command {
            Some(Commands::Status { json }) => assert!(json),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn issue_start_requires_title() {
        assert!(Cli::try_parse_from(["gww", "issue", "start", "42"]).is_err());
    }
}
