use crate::git::{count_pull_requests, run_command};
use crate::output::{print_progress, print_table};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use std::io::{self, Write};

/// Represents the statistics for a single author.
#[derive(Default, Debug, Clone)]
pub struct AuthorStats {
    pub loc: usize,
    pub commits: usize,
    pub files: HashSet<String>,
}

impl AuthorStats {
    pub fn to_json(&self) -> String {
        let files_json: Vec<String> = self.files.iter().map(|f| format!("\"{}\"", f)).collect();
        format!(
            "{{\"loc\": {}, \"commits\": {}, \"files\": [{}]}}",
            self.loc,
            self.commits,
            files_json.join(", ")
        )
    }
}

#[derive(Default, Debug, Clone)]
pub struct UserStats {
    pub tags: HashSet<String>,
    pub pull_requests: usize,
}

impl UserStats {
    pub fn to_json(&self) -> String {
        let tags_json: Vec<String> = self.tags.iter().map(|t| format!("\"{}\"", t)).collect();
        format!(
            "{{\"tags\": [{}], \"pull_requests\": {}}}",
            tags_json.join(", "),
            self.pull_requests
        )
    }
}

// A type alias for our map of statistics for readability.
pub type StatsMap = HashMap<String, AuthorStats>;

/// Gathers historical commit counts for each author from `git log`.
pub fn gather_commit_stats() -> Result<StatsMap, String> {
    let mut stats: StatsMap = HashMap::new();
    let log_output = run_command(&["log", "--no-merges", "--pretty=format:--%aN--"])?;

    for author in log_output.split("--").filter(|s| !s.is_empty()) {
        // check if is empty, if not then increment commits
        let trimmed_author = author.trim().to_string();

        if !trimmed_author.is_empty() {
            stats.entry(trimmed_author).or_default().commits += 1;
        }
    }
    Ok(stats)
}

/// Gathers LOC and file stats by running `git blame` in parallel.
pub fn gather_loc_and_file_stats() -> Result<StatsMap, String> {
    let files_to_blame: Vec<String> = run_command(&["ls-files"])?
        .lines()
        .map(String::from)
        .collect();

    let stats = Arc::new(Mutex::new(StatsMap::new()));
    let total_files = files_to_blame.len();
    let processed_files = Arc::new(Mutex::new(0));
    let start_time = Instant::now();

    thread::scope(|s| {
        for file in files_to_blame {
            let stats_clone = Arc::clone(&stats);
            let processed_clone = Arc::clone(&processed_files);

            s.spawn(move || {
                if let Ok(blame_output) =
                    run_command(&["blame", "-w", "-C", "-C", "--line-porcelain", &file])
                {
                    let mut current_author = String::new();
                    let mut author_loc_for_file = HashMap::new();

                    for line in blame_output.lines() {
                        if line.starts_with("author ") {
                            current_author = line[7..].trim().to_string();
                        } else if line.starts_with('\t') {
                            if !current_author.is_empty() {
                                *author_loc_for_file
                                    .entry(current_author.clone())
                                    .or_insert(0) += 1;
                            }
                        }
                    }

                    let mut stats_guard = stats_clone.lock().unwrap();
                    for (author, loc) in author_loc_for_file {
                        if !author.is_empty() {
                            let author_stats = stats_guard.entry(author).or_default();
                            author_stats.loc += loc;
                            author_stats.files.insert(file.clone());
                        }
                    }
                }

                let mut processed_count = processed_clone.lock().unwrap();
                *processed_count += 1;
                print_progress(*processed_count, total_files, start_time);
            });
        }
    });

    println!(); // Newline after progress bar finishes.
    let final_stats = Arc::try_unwrap(stats).unwrap().into_inner().unwrap();
    Ok(final_stats)
}

pub fn gather_user_stats(username: &str) -> Result<UserStats, String> {
    let mut user_stats = UserStats::default();

    // handle tag listing errors as empty
    let tags_output = match run_command(&["tag", "--list", "--format=%(refname:short)"]) {
        Ok(s) => s,
        Err(_) => String::new(),
    };
    for tag in tags_output.lines() {
        // If git log fails for a tag, treat as no matches for that tag.
        let log_output = run_command(&["log", tag, "--author", username, "--pretty=format:%an"])
            .unwrap_or_default();
        if !log_output.is_empty() {
            user_stats.tags.insert(tag.to_string());
        }
    }

    // If counting PR merges fails, default to 0 for resilience.
    user_stats.pull_requests = count_pull_requests(username).unwrap_or(0);

    Ok(user_stats)
}

fn tracked_text_files_head() -> Result<Vec<String>, String> {
    // tracked files (preserve order)
    let files = run_command(&["--no-pager", "ls-files"])?;
    let files: Vec<String> = files
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // text files at HEAD
    let grep = run_command(&["--no-pager", "grep", "-I", "--name-only", ".", "HEAD"])?;
    let mut text: HashSet<String> = HashSet::new();
    for mut line in grep.lines().map(|s| s.trim()) {
        if line.is_empty() {
            continue;
        }
        if let Some(stripped) = line.strip_prefix("HEAD:") {
            line = stripped;
        }
        text.insert(line.to_string());
    }

    // Intersect while preserving original order
    let filtered: Vec<String> = files.into_iter().filter(|f| text.contains(f)).collect();
    Ok(filtered)
}

/// Gather surviving LOC per author via blame --line-porcelain HEAD.
/// by_name=false groups by "Name <email>", by_name=true groups by name only.
pub fn gather_loc_and_file_statsx(by_name: bool) -> Result<StatsMap, String> {
    let files = tracked_text_files_head()?;
    let mut stats: StatsMap = HashMap::new();

    let total = files.len();
    let mut idx: usize = 0;
    let spinner = ['|', '/', '-', '\\'];

    for file in files {
        idx += 1;
        let ch = spinner[idx % spinner.len()];
        print!("\rProcessing: {}/{} {}", idx, total, ch);
        let _ = io::stdout().flush();

        let blame = run_command(&["--no-pager", "blame", "--line-porcelain", "HEAD", "--", &file]);
        if blame.is_err() {
            continue;
        }
        let blame = blame.unwrap();

        let mut current_name: Option<String> = None;
        let mut current_mail: Option<String> = None;

        for line in blame.lines() {
            if let Some(rest) = line.strip_prefix("author ") {
                current_name = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("author-mail ") {
                current_mail = Some(rest.trim().to_string());
            } else if line.starts_with('\t') {
                if let (Some(name), Some(mail)) = (&current_name, &current_mail) {
                    let key = if by_name {
                        name.clone()
                    } else {
                        format!("{} {}", name, mail)
                    };
                    let entry = stats.entry(key).or_default();
                    entry.loc += 1;
                    entry.files.insert(file.clone());
                }
            }
        }
    }

    println!();
    Ok(stats)
}

/// Gather commit counts per author via `git shortlog -s -e HEAD`.
/// by_name=false groups by "Name <email>", by_name=true groups by name only.
pub fn gather_commit_statsx(by_name: bool) -> Result<StatsMap, String> {
    let out = run_command(&["--no-pager", "shortlog", "-s", "-e", "HEAD"])?;
    let mut stats: StatsMap = HashMap::new();

    for line in out.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        // parse leading integer
        let mut idx = 0;
        while idx < l.len() && l.as_bytes()[idx].is_ascii_whitespace() {
            idx += 1;
        }
        let start_num = idx;
        while idx < l.len() && l.as_bytes()[idx].is_ascii_digit() {
            idx += 1;
        }
        if start_num == idx {
            continue;
        }
        let num_str = &l[start_num..idx];
        let commits: usize = num_str.parse().unwrap_or(0);
        let rest = l[idx..].trim();
        if rest.is_empty() {
            continue;
        }
        let key = if by_name {
            let name_part = rest.rsplit_once(" <").map(|(n, _)| n).unwrap_or(rest);
            name_part.to_string()
        } else {
            rest.to_string()
        };
        let entry = stats.entry(key).or_default();
        entry.commits += commits;
    }

    Ok(stats)
}

/// Orchestrate stats and print totals + table.
pub fn run_stats(by_name: bool) -> Result<(), String> {
    let mut commit_stats = gather_commit_statsx(by_name)?;
    let loc_stats = gather_loc_and_file_statsx(by_name)?;

    let mut final_stats = loc_stats;
    for (author, data) in commit_stats.drain() {
        final_stats.entry(author).or_default().commits = data.commits;
    }

    let total_loc: usize = final_stats.values().map(|s| s.loc).sum();
    let total_commits: usize = final_stats.values().map(|s| s.commits).sum();

    let mut all_files = HashSet::new();
    for stats in final_stats.values() {
        all_files.extend(stats.files.iter().cloned());
    }
    let total_files = all_files.len();

    let mut rows: Vec<(String, AuthorStats)> = final_stats.into_iter().collect();
    rows.sort_by(|a, b| b.1.loc.cmp(&a.1.loc));

    println!("Total commits: {}", total_commits);
    println!("Total files: {}", total_files);
    println!("Total loc: {}", total_loc);
    print_table(rows, total_loc, total_commits, total_files);
    Ok(())
}

/// Compute per-file ownership for a user.
/// - username: match against blame author (by_name) or author-mail (by_email)
/// - by_email: if true, compare normalized emails; otherwise compare author name
/// - top: max rows to return (use usize::MAX to disable)
/// - sort_pct: if true, sort by percentage desc; otherwise by user_loc desc
pub fn get_user_file_ownership(
    username: &str,
    by_email: bool,
    top: usize,
    sort_pct: bool,
) -> Result<Vec<(String, usize, usize, f32)>, String> {
    let files = tracked_text_files_head()?;
    let mut rows: Vec<(String, usize, usize, f32)> = Vec::new();

    let uname_norm = username.trim().to_string();
    // normalize email for comparison
    let email_norm = uname_norm
        .trim_matches(|c| c == '<' || c == '>')
        .to_ascii_lowercase();

    for file in files {
        let blame = run_command(&["--no-pager", "blame", "--line-porcelain", "HEAD", "--", &file]);
        if blame.is_err() {
            continue;
        }
        let blame = blame.unwrap();

        let mut current_name: Option<String> = None;
        let mut current_mail: Option<String> = None;
        let mut file_total: usize = 0;
        let mut user_loc: usize = 0;

        for line in blame.lines() {
            if let Some(rest) = line.strip_prefix("author ") {
                current_name = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("author-mail ") {
                current_mail = Some(rest.trim().to_string());
            } else if line.starts_with('\t') {
                file_total += 1;
                if let (Some(name), Some(mail)) = (&current_name, &current_mail) {
                    let is_match = if by_email {
                        let mail_norm = mail.trim_matches(|c| c == '<' || c == '>').to_ascii_lowercase();
                        mail_norm == email_norm
                    } else {
                        name == &uname_norm
                    };
                    if is_match {
                        user_loc += 1;
                    }
                }
            }
        }

        if user_loc > 0 && file_total > 0 {
            let pct = (user_loc as f32 / file_total as f32) * 100.0;
            rows.push((file, user_loc, file_total, pct));
        }
    }

    if sort_pct {
        rows.sort_by(|a, b| {
            b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| a.0.cmp(&b.0))
        });
    } else {
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal)).then_with(|| a.0.cmp(&b.0)));
    }

    if top < rows.len() {
        rows.truncate(top);
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_author_stats_default() {
        let stats = AuthorStats::default();
        assert_eq!(stats.loc, 0);
        assert_eq!(stats.commits, 0);
        assert!(stats.files.is_empty());
    }

    #[test]
    fn test_user_stats_default() {
        let stats = UserStats::default();
        assert!(stats.tags.is_empty());
        assert_eq!(stats.pull_requests, 0);
    }

    #[test]
    fn test_gather_commit_stats_runs_ok() {
        // Serialize against other CWD-mutating tests to ensure a stable repo context.
        let _guard = crate::test_sync::test_lock();
        // This test runs against the live git repository.
        let result = gather_commit_stats();
        assert!(result.is_ok());
        let stats = result.unwrap();
        // The project should have at least one commit/author.
        assert!(!stats.is_empty());
    }

    #[test]
    #[ignore] // This test is slow and prints to stdout.
    fn test_gather_loc_and_file_stats_runs_ok() {
        // This test runs against the live git repository and can be slow.
        let result = gather_loc_and_file_stats();
        assert!(result.is_ok());
        let stats = result.unwrap();
        // The project should have some stats.
        assert!(!stats.is_empty());
    }

    #[test]
    fn test_gather_user_stats_for_unknown_user() {
        // Test with a user that almost certainly doesn't exist.
        let result = gather_user_stats("a-very-unlikely-user-name-to-exist");
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.pull_requests, 0);
        assert!(stats.tags.is_empty());
    }

    #[test]
    fn test_author_stats_to_json() {
        let mut author_stats = AuthorStats::default();
        author_stats.loc = 100;
        author_stats.commits = 10;
        author_stats.files.insert("file1.rs".to_string());
        author_stats.files.insert("file2.rs".to_string());

        let json = author_stats.to_json();
        // Due to HashSet's unordered nature, we need to check for both possible orders of files.
        let expected_json1 = "{\"loc\": 100, \"commits\": 10, \"files\": [\"file1.rs\", \"file2.rs\"]}";
        let expected_json2 = "{\"loc\": 100, \"commits\": 10, \"files\": [\"file2.rs\", \"file1.rs\"]}";

        assert!(json == expected_json1 || json == expected_json2, "Actual JSON: {}", json);
    }

    #[test]
    fn test_user_stats_to_json() {
        let mut user_stats = UserStats::default();
        user_stats.pull_requests = 5;
        user_stats.tags.insert("v1.0".to_string());
        user_stats.tags.insert("v1.1".to_string());

        let json = user_stats.to_json();
        // Due to HashSet's unordered nature, we need to check for both possible orders of tags.
        let expected_json1 = "{\"tags\": [\"v1.0\", \"v1.1\"], \"pull_requests\": 5}";
        let expected_json2 = "{\"tags\": [\"v1.1\", \"v1.0\"], \"pull_requests\": 5}";

        assert!(json == expected_json1 || json == expected_json2, "Actual JSON: {}", json);
    }

    // Ownership tests (create and clean a small repo under ./.tmp-git-insights-tests)
    use std::env;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::sync::MutexGuard;

    struct TempRepo {
        _guard: MutexGuard<'static, ()>,
        old_dir: PathBuf,
        base: PathBuf,
        path: PathBuf,
    }

    impl TempRepo {
        fn new() -> Self {
            let guard = crate::test_sync::test_lock();

            let old_dir = env::current_dir().unwrap();
            let base = old_dir.join(".tmp-git-insights-tests");
            fs::create_dir_all(&base).unwrap();
            let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let path = base.join(format!("git-insights-ownership-{}", ts));
            fs::create_dir_all(&path).unwrap();
            env::set_current_dir(&path).unwrap();

            assert!(
                Command::new("git")
                    .args(["init", "-q"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .unwrap()
                    .success()
            );
            fs::write("INIT", "init\n").unwrap();
            let add_ok = Command::new("git")
                .args(["add", "."])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
                || Command::new("git")
                    .args(["add", "-A", "."])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
            assert!(add_ok, "git add failed in TempRepo::new");
            let mut c = Command::new("git");
            c.args(["-c", "commit.gpgsign=false"])
                .arg("commit")
                .arg("--no-verify")
                .arg("-q")
                .arg("-m")
                .arg("chore: init");
            c.env("GIT_AUTHOR_NAME", "Init");
            c.env("GIT_AUTHOR_EMAIL", "init@example.com");
            c.env("GIT_COMMITTER_NAME", "Init");
            c.env("GIT_COMMITTER_EMAIL", "init@example.com");
            c.stdout(Stdio::null()).stderr(Stdio::null());
            assert!(c.status().unwrap().success());

            Self { _guard: guard, old_dir, base, path }
        }
    }
    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.old_dir);
            let _ = fs::remove_dir_all(&self.path);
            // Ensure the base test directory is also removed so tests leave no trace
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    fn commit_as(name: &str, email: &str, msg: &str) {
        let add_ok = Command::new("git")
            .args(["add", "."])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            || Command::new("git")
                .args(["add", "-A", "."])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        assert!(add_ok, "git add failed");
        let mut c = Command::new("git");
        c.args(["-c", "commit.gpgsign=false"])
            .args(["-c", "core.hooksPath=/dev/null"])
            .arg("commit")
            .arg("--no-verify")
            .arg("-q")
            .arg("-m")
            .arg(msg);
        c.env("GIT_AUTHOR_NAME", name);
        c.env("GIT_AUTHOR_EMAIL", email);
        c.env("GIT_COMMITTER_NAME", name);
        c.env("GIT_COMMITTER_EMAIL", email);
        c.stdout(Stdio::null()).stderr(Stdio::null());
        assert!(c.status().unwrap().success());
    }

    #[test]
    fn test_get_user_file_ownership_by_name_and_email() {
        let _repo = TempRepo::new();

        // Alice owns README fully (4 lines total)
        fs::write("README.md", "a\nb\nc\n").unwrap();
        commit_as("Alice", "alice@example.com", "feat: add README");
        fs::OpenOptions::new()
            .append(true)
            .open("README.md")
            .unwrap()
            .write_all(b"d\n")
            .unwrap();
        commit_as("Alice", "alice@example.com", "feat: update README");

        // Bob owns src.txt fully (2 lines total)
        fs::write("src.txt", "x\ny\n").unwrap();
        commit_as("Bob", "bob@example.com", "feat: add src");

        // By name
        let rows = super::get_user_file_ownership("Alice", false, usize::MAX, false)
            .expect("ownership by name failed");
        // Expect README.md 4/4 ~100%
        let mut found_readme = false;
        for (file, u, f, pct) in &rows {
            if file == "README.md" {
                found_readme = true;
                assert_eq!(*u, 4);
                assert_eq!(*f, 4);
                assert!((*pct - 100.0).abs() < 0.01);
            }
        }
        assert!(found_readme);

        // By email
        let rows_email =
            super::get_user_file_ownership("alice@example.com", true, usize::MAX, false)
                .expect("ownership by email failed");
        let mut found_readme_email = false;
        for (file, u, f, pct) in &rows_email {
            if file == "README.md" {
                found_readme_email = true;
                assert_eq!(*u, 4);
                assert_eq!(*f, 4);
                assert!((*pct - 100.0).abs() < 0.01);
            }
        }
        assert!(found_readme_email);

        // Bob by name should show src.txt 2/2
        let rows_bob = super::get_user_file_ownership("Bob", false, usize::MAX, false)
            .expect("ownership Bob failed");
        let mut found_src = false;
        for (file, u, f, pct) in &rows_bob {
            if file == "src.txt" {
                found_src = true;
                assert_eq!(*u, 2);
                assert_eq!(*f, 2);
                assert!((*pct - 100.0).abs() < 0.01);
            }
        }
        assert!(found_src);
    }

    #[test]
    fn test_get_user_file_ownership_top_and_sort_pct() {
        let _repo = TempRepo::new();

        // Create 3 files owned by Alice with varying ownership
        fs::write("a.txt", "1\n2\n3\n").unwrap(); // 3 lines
        commit_as("Alice", "alice@example.com", "a");
        fs::write("b.txt", "1\n2\n").unwrap(); // 2 lines
        commit_as("Alice", "alice@example.com", "b");
        fs::write("c.txt", "1\n2\n3\n4\n").unwrap(); // 4 lines
        commit_as("Alice", "alice@example.com", "c");

        // sort by pct and top 2
        let rows = super::get_user_file_ownership("Alice", false, 2, true)
            .expect("ownership sort pct failed");
        // All 100% but ensure we only got top 2 rows
        assert_eq!(rows.len(), 2);
    }
}
