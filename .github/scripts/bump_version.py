import os
import re
import sys
import subprocess
from pathlib import Path

ROOT = Path(".").resolve()
CARGO_TOML = ROOT / "Cargo.toml"
CARGO_LOCK = ROOT / "Cargo.lock"


def bump_patch_in_cargo_toml(path: Path):
    txt = path.read_text(encoding="utf-8").splitlines(keepends=True)
    in_pkg = False
    changed = False
    version_re = re.compile(r'^(\s*version\s*=\s*")(\d+)\.(\d+)\.(\d+)(".*)$')
    for i, line in enumerate(txt):
        if re.match(r"^\s*\[package\]\s*$", line):
            in_pkg = True
        elif re.match(r"^\s*\[.*\]\s*$", line):
            in_pkg = False
        if in_pkg:
            m = version_re.match(line)
            if m:
                major, minor, patch = map(int, (m.group(2), m.group(3), m.group(4)))
                new_line = f"{m.group(1)}{major}.{minor}.{patch + 1}{m.group(5)}\n"
                # Preserve original line ending style
                if line.endswith("\r\n"):
                    new_line = new_line[:-1] + "\r\n"
                txt[i] = new_line
                changed = True
                break
    if not changed:
        sys.exit("Could not find version in [package] section of Cargo.toml")
    path.write_text("".join(txt), encoding="utf-8")


def run(cmd):
    return subprocess.run(cmd, shell=True)


def check_only_two_files_modified():
    out = subprocess.check_output(["git", "status", "--porcelain"], text=True)
    paths = []
    for line in out.splitlines():
        if not line.strip():
            continue
        status = line[:2]
        path = line[3:].strip()
        # Only allow modifications (staged or unstaged)
        if (
            status.strip() not in {"M", "MM", "AM", "XM", "T", "MT", "??"}
            and "D" not in status
        ):
            sys.exit(f"Unexpected git status for {path}: {status!r}")
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
        if (
            line.startswith("+++ ")
            or line.startswith("--- ")
            or line.startswith("@@")
            or line.startswith("diff ")
        ):
            continue
        if line.startswith("+"):
            plus_lines.append(line[1:])
        elif line.startswith("-"):
            minus_lines.append(line[1:])
    # Ignore empty-context noise (should be exactly one add and one remove)
    if len(plus_lines) != 1 or len(minus_lines) != 1:
        sys.exit(f"{file_path}: expected exactly one added and one removed line")
    return minus_lines[0].strip(), plus_lines[0].strip()


def main():
    if not CARGO_TOML.exists():
        sys.exit("Cargo.toml not found next to this script")

    # 1) bump patch version in Cargo.toml
    bump_patch_in_cargo_toml(CARGO_TOML)

    # 2) build
    rc = os.system("cargo build")
    if rc != 0:
        sys.exit(rc)

    # 3) ensure only Cargo.toml and Cargo.lock changed
    check_only_two_files_modified()

    # 4) ensure the exact same single-line change in both files
    old_toml, new_toml = extract_single_line_change("Cargo.toml")
    old_lock, new_lock = extract_single_line_change("Cargo.lock")
    if new_toml != new_lock or old_toml != old_lock:
        sys.exit("Changed line in Cargo.toml and Cargo.lock are not identical")


if __name__ == "__main__":
    main()
