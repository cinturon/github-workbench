use std::io::{self, BufRead, IsTerminal, Write};

use workbench_application::AppError;

pub fn confirm(plan: &str, yes: bool) -> Result<bool, AppError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    confirm_with_io(
        plan,
        yes,
        stdin.is_terminal(),
        stdin.lock(),
        stdout.lock(),
    )
}

fn confirm_with_io<R, W>(
    plan: &str,
    yes: bool,
    is_terminal: bool,
    mut input: R,
    mut output: W,
) -> Result<bool, AppError>
where
    R: BufRead,
    W: Write,
{
    if yes {
        return Ok(true);
    }
    if !is_terminal {
        return Err(AppError::Usage {
            message: "refusing to execute without --yes because stdin is not a TTY".into(),
        });
    }

    writeln!(output, "{plan}").map_err(|error| AppError::Io {
        path: "stdout".into(),
        detail: error.to_string(),
    })?;
    write!(output, "Proceed? [y/N] ").map_err(|error| AppError::Io {
        path: "stdout".into(),
        detail: error.to_string(),
    })?;
    output.flush().map_err(|error| AppError::Io {
        path: "stdout".into(),
        detail: error.to_string(),
    })?;

    let mut answer = String::new();
    input
        .read_line(&mut answer)
        .map_err(|error| AppError::Io {
            path: "stdin".into(),
            detail: error.to_string(),
        })?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    #[test]
    fn yes_bypasses_terminal_requirement_and_prompt() {
        let mut output = Vec::new();
        let confirmed =
            confirm_with_io("Plan text", true, false, Cursor::new(""), &mut output).unwrap();

        assert!(confirmed);
        assert!(output.is_empty());
    }

    #[test]
    fn non_terminal_without_yes_is_invalid_usage() {
        let error =
            confirm_with_io("Plan text", false, false, Cursor::new("yes\n"), Vec::new())
                .unwrap_err();

        assert_eq!(
            error,
            AppError::Usage {
                message: "refusing to execute without --yes because stdin is not a TTY".into()
            }
        );
    }

    #[test]
    fn terminal_prints_plan_and_accepts_yes_case_insensitively() {
        let mut output = Vec::new();
        let confirmed =
            confirm_with_io("Plan text", false, true, Cursor::new("YES\n"), &mut output).unwrap();

        assert!(confirmed);
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Plan text\nProceed? [y/N] "
        );
    }

    #[test]
    fn terminal_defaults_to_decline() {
        let confirmed =
            confirm_with_io("Plan text", false, true, Cursor::new("\n"), Vec::new()).unwrap();

        assert!(!confirmed);
    }
}
