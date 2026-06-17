use std::fmt;
use std::path::Path;
use std::process::{Command, Output};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GitError {
    NotFound,
    StartFailed,
    NothingToCommit,
    Failed { action: String },
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::NotFound => write!(f, "git executable not found"),
            GitError::StartFailed => write!(f, "could not start git"),
            GitError::NothingToCommit => write!(f, "nothing to commit"),
            GitError::Failed { action } => write!(f, "git {} exited with an error", action),
        }
    }
}

pub(crate) fn run(repo_root: &Path, args: &[&str]) -> Result<(), GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GitError::NotFound
            } else {
                GitError::StartFailed
            }
        })?;

    output_result(args, output)
}

fn output_result(args: &[&str], output: Output) -> Result<(), GitError> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    let stdout = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
    if args.first() == Some(&"commit")
        && (stderr.contains("nothing to commit") || stdout.contains("nothing to commit"))
    {
        return Err(GitError::NothingToCommit);
    }

    Err(GitError::Failed {
        action: args.first().copied().unwrap_or("command").to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::ExitStatus;

    #[cfg(unix)]
    fn status(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }

    #[cfg(windows)]
    fn status(code: u32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code)
    }

    #[test]
    fn commit_noop_is_structured_error() {
        let err = output_result(
            &["commit", "-m", "sync"],
            Output {
                status: status(1),
                stdout: b"nothing to commit, working tree clean".to_vec(),
                stderr: Vec::new(),
            },
        )
        .unwrap_err();

        assert_eq!(err, GitError::NothingToCommit);
    }
}
