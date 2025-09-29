use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Simple author representation for generating commits.
#[derive(Clone, Debug)]
pub struct Author {
    pub name: String,
    pub email: String,
}

impl Author {
    pub fn new<N: Into<String>, E: Into<String>>(name: N, email: E) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
        }
    }
}

/// A temporary git repository under ./_tmp that cleans itself up on drop.
#[derive(Debug)]
pub struct TestRepo {
    pub path: PathBuf,
}

impl TestRepo {
    /// Create a unique directory under ./_tmp.
    fn create_unique_tmp_dir() -> Result<PathBuf, String> {
        let cwd = env::current_dir().map_err(|e| format!("cwd error: {}", e))?;
        let tmp_root = cwd.join("_tmp");
        if !tmp_root.exists() {
            fs::create_dir_all(&tmp_root)
                .map_err(|e| format!("failed to create _tmp dir: {}", e))?;
        }
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("time error: {}", e))?
            .as_millis();

        let pid = std::process::id();
        let unique = format!("repo-{}-{}", millis, pid);

        let repo_dir = tmp_root.join(unique);
        fs::create_dir_all(&repo_dir).map_err(|e| format!("failed to create repo dir: {}", e))?;

        Ok(repo_dir)
    }

    /// Initialize a new git repository under ./_tmp with default branch 'main'.
    pub fn init() -> Result<Self, String> {
        let path = Self::create_unique_tmp_dir()?;

        // Use -c to set default branch without relying on global config.
        let status = Command::new("git")
            .arg("-c")
            .arg("init.defaultBranch=main")
            .arg("init")
            .arg("-q")
            .current_dir(&path)
            .status()
            .map_err(|e| format!("failed to run git init: {}", e))?;

        if !status.success() {
            return Err("git init failed".into());
        }

        // Set a dummy config to avoid prompts/warnings. These are defaults and
        // will be overridden per-commit via env for multiple authors.
        let status = Command::new("git")
            .args(["config", "user.name", "Temp User"])
            .current_dir(&path)
            .status()
            .map_err(|e| format!("failed to set user.name: {}", e))?;
        if !status.success() {
            return Err("git config user.name failed".into());
        }

        let status = Command::new("git")
            .args(["config", "user.email", "temp@test_git_insights.com"])
            .current_dir(&path)
            .status()
            .map_err(|e| format!("failed to set user.email: {}", e))?;
        if !status.success() {
            return Err("git config user.email failed".into());
        }

        Ok(Self { path })
    }

    fn run_git(&self, args: &[&str]) -> Result<Output, String> {
        Command::new("git")
            .args(args)
            .current_dir(&self.path)
            .output()
            .map_err(|e| format!("failed to run git {:?}: {}", args, e))
    }

    fn run_git_ok(&self, args: &[&str]) -> Result<(), String> {
        let out = self.run_git(args)?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!(
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            ))
        }
    }

    /// Write or append a line to a file in the repo.
    fn append_line<P: AsRef<Path>>(&self, path: P, line: &str) -> Result<(), String> {
        let file_path = self.path.join(path);
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create parent dirs: {}", e))?;
            }
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|e| format!("failed to open file {:?}: {}", file_path, e))?;
        writeln!(f, "{}", line).map_err(|e| format!("failed to write: {}", e))?;
        Ok(())
    }

    /// Create a sequence of commits rotating through the provided authors.
    ///
    /// - num_commits: number of commits to create.
    /// - authors: at least one author is required.
    /// - files_spread: number of files to rotate modifications across (>=1).
    ///
    /// Commits will touch files named file0.txt..file{files_spread-1}.txt.
    pub fn seed_commits(
        &self,
        num_commits: usize,
        authors: &[Author],
        files_spread: usize,
    ) -> Result<(), String> {
        if num_commits == 0 {
            return Ok(());
        }
        if authors.is_empty() {
            return Err("authors must not be empty".into());
        }
        let files_spread = files_spread.max(1);

        let readme = self.path.join("README.seed.md");
        let mut f =
            File::create(&readme).map_err(|e| format!("failed to create seed file: {}", e))?;
        writeln!(f, "# Seed repo").map_err(|e| format!("failed to write seed: {}", e))?;
        drop(f);

        self.run_git_ok(&["add", "."])?;
        let mut cmd = Command::new("git");
        cmd.arg("commit")
            .arg("-q")
            .arg("-m")
            .arg("chore(seed): initial file");
        cmd.current_dir(&self.path);
        cmd.env("GIT_AUTHOR_NAME", "Seeder");
        cmd.env("GIT_AUTHOR_EMAIL", "seed@test_git_insights.com");
        cmd.env("GIT_COMMITTER_NAME", "Seeder");
        cmd.env("GIT_COMMITTER_EMAIL", "seed@test_git_insights.com");
        cmd.env("GIT_AUTHOR_DATE", date_seconds_offset_from_now(0).as_str());
        cmd.env(
            "GIT_COMMITTER_DATE",
            date_seconds_offset_from_now(0).as_str(),
        );
        let out = cmd.output().map_err(|e| format!("commit error: {}", e))?;
        if !out.status.success() {
            return Err(format!(
                "initial commit failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }

        for i in 0..num_commits {
            let a = &authors[i % authors.len()];
            let file_ix = i % files_spread;
            let file_name = format!("file{}.txt", file_ix);
            let line = format!("line {} by {} <{}>", i + 1, a.name, a.email);
            self.append_line(&file_name, &line)?;

            self.run_git_ok(&["add", "--all"])?;

            let msg = format!("feat: commit {} by {} <{}>", i + 1, a.name, a.email);
            let mut c = Command::new("git");
            c.arg("commit")
                .arg("-q")
                .arg("-m")
                .arg(msg)
                .current_dir(&self.path);
            c.env("GIT_AUTHOR_NAME", &a.name);
            c.env("GIT_AUTHOR_EMAIL", &a.email);
            c.env("GIT_COMMITTER_NAME", &a.name);
            c.env("GIT_COMMITTER_EMAIL", &a.email);

            let secs = 60 * (i as u64 + 1);
            let date = date_seconds_offset_from_now(secs);
            c.env("GIT_AUTHOR_DATE", &date);
            c.env("GIT_COMMITTER_DATE", &date);

            let out = c.output().map_err(|e| format!("commit error: {}", e))?;
            if !out.status.success() {
                return Err(format!(
                    "commit {} failed: {}",
                    i + 1,
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
        }

        Ok(())
    }

    /// Create a sequence of commits with explicit start time and step (seconds).
    ///
    /// Commits will be authored/committed by rotating authors, touching files across `files_spread`,
    /// and timestamps will be `start_epoch + i * step_secs` (i=1..=num_commits), with an initial seed commit at start_epoch.
    pub fn seed_commits_with_schedule(
        &self,
        num_commits: usize,
        authors: &[Author],
        files_spread: usize,
        start_epoch: u64,
        step_secs: u64,
    ) -> Result<(), String> {
        if authors.is_empty() {
            return Err("authors must not be empty".into());
        }
        let head_exists = Command::new("git")
            .args(["rev-parse", "--verify", "HEAD"])
            .current_dir(&self.path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        let date0 = format!("{} +0000", start_epoch);
        if !head_exists {
            let readme = self.path.join("README.seed.md");
            let mut f =
                File::create(&readme).map_err(|e| format!("failed to create seed file: {}", e))?;
            writeln!(f, "# Seed repo (scheduled)")
                .map_err(|e| format!("failed to write seed: {}", e))?;
            drop(f);

            self.run_git_ok(&["add", "."])?;
            let mut c0 = Command::new("git");
            c0.arg("commit")
                .arg("-q")
                .arg("-m")
                .arg("chore(seed): initial file (scheduled)")
                .current_dir(&self.path);
            c0.env("GIT_AUTHOR_NAME", "Seeder");
            c0.env("GIT_AUTHOR_EMAIL", "seed@test_git_insights.com");
            c0.env("GIT_COMMITTER_NAME", "Seeder");
            c0.env("GIT_COMMITTER_EMAIL", "seed@test_git_insights.com");
            c0.env("GIT_AUTHOR_DATE", &date0);
            c0.env("GIT_COMMITTER_DATE", &date0);

            let out = c0
                .output()
                .map_err(|e| format!("seed commit error: {}", e))?;
            if !out.status.success() {
                return Err(format!(
                    "seed commit failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
        } else {
            let mut amend = Command::new("git");
            amend
                .arg("commit")
                .arg("--amend")
                .arg("--no-edit")
                .current_dir(&self.path);
            amend.env("GIT_AUTHOR_DATE", &date0);
            amend.env("GIT_COMMITTER_DATE", &date0);
            let out = amend
                .output()
                .map_err(|e| format!("amend seed date error: {}", e))?;
            if !out.status.success() {
                return Err(format!(
                    "amend seed date failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
        }

        if num_commits == 0 {
            return Ok(());
        }
        let files_spread = files_spread.max(1);

        for i in 0..num_commits {
            let a = &authors[i % authors.len()];
            let file_ix = i % files_spread;
            let file_name = format!("file{}.txt", file_ix);
            let line = format!("line {} by {} <{}>", i + 1, a.name, a.email);
            self.append_line(&file_name, &line)?;
            self.run_git_ok(&["add", "--all"])?;

            let msg = format!("feat: commit {} by {} <{}>", i + 1, a.name, a.email);
            let ts = start_epoch.saturating_add(step_secs.saturating_mul((i as u64) + 1));
            let date = format!("{} +0000", ts);

            let mut c = Command::new("git");
            c.arg("commit")
                .arg("-q")
                .arg("-m")
                .arg(msg)
                .current_dir(&self.path);
            c.env("GIT_AUTHOR_NAME", &a.name);
            c.env("GIT_AUTHOR_EMAIL", &a.email);
            c.env("GIT_COMMITTER_NAME", &a.name);
            c.env("GIT_COMMITTER_EMAIL", &a.email);
            c.env("GIT_AUTHOR_DATE", &date);
            c.env("GIT_COMMITTER_DATE", &date);

            let out = c.output().map_err(|e| format!("commit error: {}", e))?;
            if !out.status.success() {
                return Err(format!(
                    "scheduled commit {} failed: {}",
                    i + 1,
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
        }
        Ok(())
    }

    /// Commit specific content at an exact epoch timestamp with given identity.
    pub fn commit_with_epoch(
        &self,
        name: &str,
        email: &str,
        file: &str,
        content: &str,
        ts: u64,
    ) -> Result<(), String> {
        self.append_line(file, content)?;

        self.run_git_ok(&["add", "--all"])?;

        let (y, mth, d) = crate::code_frequency::ymd_from_unix(ts);
        let h = (ts / 3_600) % 24;
        let min = (ts / 60) % 60;
        let sec = ts % 60;
        let date = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} +0000",
            y, mth, d, h, min, sec
        );

        let mut c = Command::new("git");
        c.arg("commit")
            .arg("-q")
            .arg("--allow-empty")
            .arg("-m")
            .arg("test")
            .current_dir(&self.path);
        c.env("GIT_AUTHOR_NAME", name);
        c.env("GIT_AUTHOR_EMAIL", email);
        c.env("GIT_COMMITTER_NAME", name);
        c.env("GIT_COMMITTER_EMAIL", email);
        c.env("GIT_AUTHOR_DATE", &date);
        c.env("GIT_COMMITTER_DATE", &date);

        let out = c
            .output()
            .map_err(|e| format!("commit spawn error: {}", e))?;
        if !out.status.success() {
            return Err(format!(
                "commit failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(())
    }

    /// Convenience routine: create a repo and seed N commits over given authors.
    pub fn create_with_commits(
        num_commits: usize,
        authors: &[Author],
        files_spread: usize,
    ) -> Result<Self, String> {
        let repo = Self::init()?;
        repo.seed_commits(num_commits, authors, files_spread)?;
        Ok(repo)
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn date_seconds_offset_from_now(secs: u64) -> String {
    let now = SystemTime::now();
    let target = now + Duration::from_secs(secs);
    let epoch = target
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();
    epoch.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::run_command;
    use crate::test_sync::test_lock;

    #[test]
    fn creates_repo_with_multiple_authors_and_commits() {
        let _guard = test_lock();

        let authors = vec![
            Author::new("Alice", "alice@test_git_insights.com"),
            Author::new("Bob", "bob@test_git_insights.com"),
            Author::new("Carol", "carol@test_git_insights.com"),
        ];
        let commits = 9usize;
        let repo = TestRepo::create_with_commits(commits, &authors, 3)
            .expect("failed to create seeded repo");

        let count = run_command(&[
            "-C",
            repo.path.to_str().unwrap(),
            "rev-list",
            "--count",
            "HEAD",
        ])
        .expect("git rev-list failed");
        let count_num: usize = count.trim().parse().unwrap();
        assert_eq!(count_num, commits + 1, "includes the initial seed commit");

        let emails = run_command(&[
            "-C",
            repo.path.to_str().unwrap(),
            "--no-pager",
            "log",
            "--pretty=format:%ae",
        ])
        .expect("git log failed");
        for a in &authors {
            assert!(
                emails.contains(&a.email),
                "expected author email {} in git log output:\n{}",
                a.email,
                emails
            );
        }
    }

    use crate::code_frequency::{run_code_frequency_with_options, HeatmapKind};
    use crate::stats::{gather_commit_statsx, gather_loc_and_file_statsx};
    use crate::visualize::{run_heatmap_with_options, run_timeline};

    #[test]
    fn multi_year_schedule_end_to_end() {
        let _guard = test_lock();

        let authors = vec![
            Author::new("Anna", "anna@test_git_insights.com"),
            Author::new("Ben", "ben@test_git_insights.com"),
        ];
        let repo = TestRepo::init().expect("init repo");
        let start_epoch = 1_577_836_800u64; // 2020-01-01 00:00:00 UTC
        let week: u64 = 7 * 86_400;
        repo.seed_commits_with_schedule(110, &authors, 4, start_epoch, week)
            .expect("seed schedule failed");

        let emails = run_command(&[
            "-C",
            repo.path.to_str().unwrap(),
            "--no-pager",
            "log",
            "--pretty=%ae",
        ])
        .expect("git log failed");
        for e in emails.lines() {
            assert!(
                e.ends_with("@test_git_insights.com"),
                "non-test email encountered: {}",
                e
            );
        }

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo.path).expect("cd into repo");

        run_timeline(104).expect("timeline e2e ok");
        run_heatmap_with_options(Some(104), false).expect("heatmap e2e ok");
        run_code_frequency_with_options(None, Some(HeatmapKind::DowByHod), Some(104), false, false)
            .expect("code frequency e2e ok");

        std::env::set_current_dir(old).expect("restore cwd");
    }

    #[test]
    fn stats_end_to_end_on_seeded_repo() {
        let _guard = test_lock();

        let authors = vec![
            Author::new("Dana", "dana@test_git_insights.com"),
            Author::new("Elle", "elle@test_git_insights.com"),
        ];
        let repo = TestRepo::init().expect("init repo");
        let start_epoch = 1_609_459_200u64; // 2021-01-01 00:00:00 UTC
        let month_ish: u64 = 30 * 86_400; // approx month
        repo.seed_commits_with_schedule(24, &authors, 5, start_epoch, month_ish)
            .expect("seed schedule failed");

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo.path).expect("cd into repo");

        let by_name = gather_commit_statsx(true).expect("commit stats by name");
        assert!(
            by_name.keys().any(|k| k == "Dana"),
            "expected Dana in commit stats, got: {:?}",
            by_name.keys().collect::<Vec<_>>()
        );
        assert!(
            by_name.keys().any(|k| k == "Elle"),
            "expected Elle in commit stats, got: {:?}",
            by_name.keys().collect::<Vec<_>>()
        );

        let loc = gather_loc_and_file_statsx(true).expect("loc stats by name");
        assert!(
            !loc.is_empty(),
            "expected non-empty LOC stats for seeded repo"
        );
        let dana_loc = loc.get("Dana").map(|s| s.loc).unwrap_or(0);
        let elle_loc = loc.get("Elle").map(|s| s.loc).unwrap_or(0);
        assert!(
            dana_loc + elle_loc > 0,
            "expected LOC attributed to authors; got Dana={}, Elle={}",
            dana_loc,
            elle_loc
        );

        // Restore CWD.
        std::env::set_current_dir(old).expect("restore cwd");
    }

    #[test]
    fn collect_commit_timestamps_from_temp_repo_moved() {
        let _guard = test_lock();
        let repo = TestRepo::init().expect("init repo");
        let t1 = 1_696_118_400u64; // 2023-10-01 00:00:00 UTC
        let t2 = 1_696_204_800u64; // 2023-10-02 00:00:00 UTC

        repo.commit_with_epoch("Alice", "alice@test_git_insights.com", "a.txt", "a\n", t1)
            .expect("commit t1");
        repo.commit_with_epoch("Bob", "bob@test_git_insights.com", "a.txt", "b\n", t2)
            .expect("commit t2");

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo.path).expect("cd");

        let ts = crate::visualize::collect_commit_timestamps().expect("collect");
        assert!(ts.iter().any(|&x| x == t1), "missing t1");
        assert!(ts.iter().any(|&x| x == t2), "missing t2");

        std::env::set_current_dir(old).ok();
    }

    #[test]
    fn run_timeline_and_heatmap_end_to_end_moved() {
        let _guard = test_lock();
        let repo = TestRepo::init().expect("init repo");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let day = 86_400;
        let t_now = now - (now % day);

        repo.commit_with_epoch("X", "x@test_git_insights.com", "x.txt", "x\n", t_now)
            .expect("c1");
        repo.commit_with_epoch(
            "Y",
            "y@test_git_insights.com",
            "x.txt",
            "y\n",
            t_now + 3_600,
        )
        .expect("c2");

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo.path).expect("cd");

        crate::visualize::run_timeline(4).expect("timeline ok");
        crate::visualize::run_heatmap().expect("heatmap ok");

        std::env::set_current_dir(old).ok();
    }

    #[test]
    fn stats_ownership_tests_moved() {
        let _guard = test_lock();
        let repo = TestRepo::init().expect("init repo");

        fs::write(repo.path.join("README.md"), "a\nb\nc\n").unwrap();
        repo.commit_with_epoch(
            "Alice",
            "alice@test_git_insights.com",
            "README.md",
            "d",
            1_700_000_000,
        )
        .expect("commit alice 1");
        repo.commit_with_epoch(
            "Alice",
            "alice@test_git_insights.com",
            "README.md",
            "e",
            1_700_000_100,
        )
        .expect("commit alice 2");

        fs::write(repo.path.join("src.txt"), "x\ny\n").unwrap();
        repo.commit_with_epoch(
            "Bob",
            "bob@test_git_insights.com",
            "src.txt",
            "z\n",
            1_700_000_200,
        )
        .expect("commit bob");

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo.path).expect("cd");

        let rows = crate::stats::get_user_file_ownership("Alice", false, usize::MAX, false)
            .expect("ownership by name failed");
        let mut found_readme = false;
        for (file, u, f, pct) in &rows {
            if file == "README.md" {
                found_readme = true;
                assert_eq!(*u, 5); // 3 initial + 2 appends in this simplified flow
                assert_eq!(*f, 5);
                assert!((*pct - 100.0).abs() < 0.01);
            }
        }
        assert!(found_readme);

        let rows_email = crate::stats::get_user_file_ownership(
            "alice@test_git_insights.com",
            true,
            usize::MAX,
            false,
        )
        .expect("ownership by email failed");
        assert!(rows_email.iter().any(|(f, _, _, _)| f == "README.md"));

        let rows_top = crate::stats::get_user_file_ownership("Alice", false, 2, true)
            .expect("ownership sort pct failed");
        assert!(rows_top.len() <= 2);

        std::env::set_current_dir(old).ok();
    }

    #[test]
    fn cf_histogram_hod_moved() {
        let _guard = test_lock();
        let repo = TestRepo::init().expect("init repo");
        let base_day = 20 * 86_400; // arbitrary epoch day

        repo.commit_with_epoch(
            "Alice",
            "alice@test_git_insights.com",
            "a.txt",
            "a\n",
            base_day + 0,
        )
        .expect("c1");
        repo.commit_with_epoch(
            "Bob",
            "bob@test_git_insights.com",
            "b.txt",
            "b\n",
            base_day + 13 * 3_600,
        )
        .expect("c2");

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo.path).expect("cd");

        run_code_frequency_with_options(None, None, None, false, false).expect("cf hod ok");

        std::env::set_current_dir(old).ok();
    }

    #[test]
    fn cf_histogram_table_from_temp_repo_moved() {
        let _guard = test_lock();
        let repo = TestRepo::init().expect("init repo");
        let base_day = 30 * 86_400;

        repo.commit_with_epoch("A", "a@test_git_insights.com", "a.txt", "a\n", base_day + 0)
            .expect("c1");
        repo.commit_with_epoch(
            "B",
            "b@test_git_insights.com",
            "b.txt",
            "b\n",
            base_day + 13 * 3_600,
        )
        .expect("c2");

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo.path).expect("cd");

        run_code_frequency_with_options(None, None, None, false, true).expect("cf table hod ok");

        std::env::set_current_dir(old).ok();
    }

    #[test]
    fn cf_heatmap_table_from_temp_repo_moved() {
        let _guard = test_lock();
        let repo = TestRepo::init().expect("init repo");
        let base_day = 40 * 86_400;

        repo.commit_with_epoch(
            "C",
            "c@test_git_insights.com",
            "c.txt",
            "c\n",
            base_day + 5 * 3_600,
        )
        .expect("c1");
        repo.commit_with_epoch(
            "D",
            "d@test_git_insights.com",
            "d.txt",
            "d\n",
            base_day + 23 * 3_600,
        )
        .expect("c2");

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo.path).expect("cd");

        run_code_frequency_with_options(None, Some(HeatmapKind::DowByHod), None, false, true)
            .expect("cf heatmap table ok");

        std::env::set_current_dir(old).ok();
    }
}
