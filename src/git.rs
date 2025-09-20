use std::process::{Command, Stdio};

/// Executes a Git command and returns its stdout if successful.
pub fn run_command(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git").args(args).output();
    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
        Err(e) => Err(format!("Failed to execute git command: {}", e)),
    }
}

/// Checks if the `git` command is available in the system's PATH.
pub fn is_git_installed() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_or(false, |s| s.success())
}

/// Counts the number of merged pull requests for a given author.
pub fn count_pull_requests(author: &str) -> Result<usize, String> {
    let log_output = run_command(&[
        "log",
        "--merges",
        "--author",
        author,
        "--pretty=format:%s", // %s gets the subject of the commit
    ])?;

    let pr_merges = log_output
        .lines()
        .filter(|line| {
            line.starts_with("Merge pull request #")
                || line.starts_with("Merge branch '")
                || line.starts_with("Merged in")
        })
        .count();

    Ok(pr_merges)
}

/// Checks if the current directory is within a Git repository.
pub fn is_in_git_repo() -> bool {
    Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_or(false, |s| s.success())
}
