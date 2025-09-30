import subprocess
import sys

TEST_DOMAINS = ["@test_git_insights.com", "@example.com"]


def run_git(args):
    return subprocess.run(
        ["git"] + args, capture_output=True, text=True, check=True
    ).stdout.strip()


def main():
    try:
        run_git(["rev-parse", "--is-inside-work-tree"])
    except subprocess.CalledProcessError:
        sys.exit("Error: Not a git repository")

    log = run_git(["log", "--pretty=format:%H %ae"])
    if not log:
        sys.exit("No commits found.")

    for line in log.splitlines():
        commit, email = line.split(maxsplit=1)
        if any(email.endswith(domain) for domain in TEST_DOMAINS):
            raise Exception(f"Testing Email {email} found in commit {commit}")


if __name__ == "__main__":
    main()
