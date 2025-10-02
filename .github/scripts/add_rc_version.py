#!/usr/bin/env python3
import os
import re
import sys
import subprocess
import json
import argparse
from datetime import datetime, timezone
from urllib.request import Request, urlopen
from urllib.error import HTTPError
from pathlib import Path

ROOT = Path(".").resolve()
CARGO_TOML = ROOT / "Cargo.toml"
CARGO_LOCK = ROOT / "Cargo.lock"


def set_rc_in_cargo_toml(path: Path) -> str:
    txt = path.read_text(encoding="utf-8").splitlines(keepends=True)
    in_pkg = False
    changed = False
    ts = datetime.now(timezone.utc).strftime("%Y%m%d%H%M%S")
    version_re = re.compile(r'^(\s*version\s*=\s*")(\d+\.\d+\.\d+)(?:[-+].*)?(".*)$')
    new_version = None

    for i, line in enumerate(txt):
        if re.match(r"^\s*\[package\]\s*$", line):
            in_pkg = True
        elif re.match(r"^\s*\[.*\]\s*$", line):
            in_pkg = False
        if in_pkg:
            m = version_re.match(line)
            if m:
                base = m.group(2)
                new_version = f"{base}-rc.{ts}"
                new_line = f"{m.group(1)}{new_version}{m.group(3)}\n"
                if line.endswith("\r\n"):
                    new_line = new_line[:-1] + "\r\n"
                txt[i] = new_line
                changed = True
                break

    if not changed or not new_version:
        sys.exit("Could not find version in [package] section of Cargo.toml")

    path.write_text("".join(txt), encoding="utf-8")
    return new_version


def run(cmd):
    return subprocess.run(cmd, shell=True, check=False)


def check_only_two_files_modified():
    out = subprocess.check_output(["git", "status", "--porcelain"], text=True)
    paths = []
    for line in out.splitlines():
        if not line.strip():
            continue
        path = line[3:].strip()
        paths.append(path)
    allowed = {"Cargo.toml", "Cargo.lock"}
    if set(paths) != allowed:
        sys.exit(f"Only Cargo.toml and Cargo.lock may change (got: {set(paths)})")


def extract_single_line_change(file_path: str):
    diff = subprocess.check_output(
        ["git", "diff", "--unified=0", "--", file_path], text=True, errors="replace"
    )
    plus_lines, minus_lines = [], []
    for line in diff.splitlines():
        if line.startswith(("+++", "---", "@@", "diff ")):
            continue
        if line.startswith("+"):
            plus_lines.append(line[1:])
        elif line.startswith("-"):
            minus_lines.append(line[1:])
    if len(plus_lines) != 1 or len(minus_lines) != 1:
        sys.exit(f"{file_path}: expected exactly one added and one removed line")
    return minus_lines[0].strip(), plus_lines[0].strip()


def read_version_from_toml_full(path: Path) -> str:
    in_pkg = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if re.match(r"^\s*\[package\]\s*$", line):
            in_pkg = True
            continue
        if re.match(r"^\s*\[.*\]\s*$", line):
            in_pkg = False
        if in_pkg:
            m = re.match(r'\s*version\s*=\s*"([^"]+)"', line)
            if m:
                return m.group(1)
    sys.exit("version not found in Cargo.toml")


def git(*args):
    r = subprocess.run(["git", *args], check=False, text=True, capture_output=True)
    if r.returncode != 0:
        sys.stderr.write(r.stderr)
        sys.exit(r.returncode)
    return r.stdout.strip()


def github_api(token: str, method: str, url: str, payload: dict):
    data = json.dumps(payload).encode("utf-8")
    req = Request(url, data=data, method=method)
    req.add_header("Authorization", f"Bearer {token}")
    req.add_header("Accept", "application/vnd.github+json")
    req.add_header("Content-Type", "application/json")
    try:
        with urlopen(req) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except HTTPError as e:
        if e.code == 422:  # PR already exists
            return None
        sys.stderr.write(e.read().decode("utf-8"))
        sys.exit(e.code)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--create-pull-request",
        action="store_true",
        help="Also push branch and open PR on GitHub",
    )
    args = parser.parse_args()

    if not CARGO_TOML.exists():
        sys.exit("Cargo.toml not found")

    new_version = set_rc_in_cargo_toml(CARGO_TOML)
    if os.system("cargo build") != 0:
        sys.exit(1)

    check_only_two_files_modified()
    old_toml, new_toml = extract_single_line_change("Cargo.toml")
    old_lock, new_lock = extract_single_line_change("Cargo.lock")
    if new_toml != new_lock or old_toml != old_lock:
        sys.exit("Changed line in Cargo.toml and Cargo.lock are not identical")

    version = read_version_from_toml_full(CARGO_TOML)
    branch = f"chore/rc-v{version}"

    actor = os.environ.get("GITHUB_ACTOR", "github-actions[bot]")
    git("config", "user.name", actor)
    git("config", "user.email", f"{actor}@users.noreply.github.com")

    existing = run(f"git rev-parse --verify {branch}")
    if existing.returncode != 0:
        git("checkout", "-b", branch)
    else:
        git("checkout", branch)

    git("add", "Cargo.toml", "Cargo.lock")
    status = git("status", "--porcelain")
    if status:
        git("commit", "-m", f"rc: Cargo to v{version}")

    if args.create_pull_request:
        run(f"git push --set-upstream origin {branch}")

        token = os.environ.get("GITHUB_TOKEN")
        repo = os.environ.get("GITHUB_REPOSITORY")
        base = os.environ.get("GITHUB_REF_NAME") or "main"
        if not (token and repo):
            sys.exit("Missing GITHUB_TOKEN or GITHUB_REPOSITORY")

        api_url = f"https://api.github.com/repos/{repo}/pulls"
        github_api(
            token,
            "POST",
            api_url,
            {
                "title": f"v{version}",
                "head": branch,
                "base": base,
                "body": f"Automated rc bump to v{version}.",
            },
        )


if __name__ == "__main__":
    main()
