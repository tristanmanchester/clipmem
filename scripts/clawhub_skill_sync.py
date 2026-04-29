#!/usr/bin/env python3
"""Check and publish the packaged clipboard-memory skill on ClawHub."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


DEFAULT_SLUG = "clipboard-memory"
DEFAULT_SKILL_DIR = Path("extras/openclaw/clipboard-memory")
DEFAULT_TIMEOUT_SECONDS = 600
DEFAULT_POLL_INTERVAL_SECONDS = 15
VERSION_RE = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")


class SyncError(RuntimeError):
    """A user-facing failure that should be printed without a traceback."""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Check or publish the packaged clipboard-memory skill on ClawHub."
    )
    parser.add_argument("mode", choices=("check", "publish"))
    parser.add_argument("--slug", default=DEFAULT_SLUG)
    parser.add_argument("--skill-dir", type=Path, default=DEFAULT_SKILL_DIR)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--timeout-seconds", type=int, default=DEFAULT_TIMEOUT_SECONDS
    )
    parser.add_argument(
        "--poll-interval-seconds",
        type=int,
        default=DEFAULT_POLL_INTERVAL_SECONDS,
    )
    return parser.parse_args()


def run_command(args: list[str]) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            args,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
    except FileNotFoundError as exc:
        raise SyncError(f"required command not found: {args[0]}") from exc


def parse_json_from_output(output: str) -> dict[str, Any]:
    start = output.find("{")
    if start == -1:
        raise SyncError(f"expected JSON output, got:\n{output.strip()}")

    try:
        parsed = json.loads(output[start:])
    except json.JSONDecodeError as exc:
        raise SyncError(f"failed to parse ClawHub JSON output: {exc}") from exc

    if not isinstance(parsed, dict):
        raise SyncError("expected ClawHub JSON output to be an object")
    return parsed


def clawhub_json(args: list[str]) -> dict[str, Any]:
    result = run_command(["clawhub", *args])
    if result.returncode != 0:
        raise SyncError(result.stdout.strip() or "clawhub command failed")
    return parse_json_from_output(result.stdout)


def semver_key(version: str) -> tuple[int, int, int]:
    match = VERSION_RE.match(version)
    if not match:
        raise SyncError(f"unsupported non-release semver: {version!r}")
    return tuple(int(part) for part in match.groups())


def compare_versions(left: str, right: str) -> int:
    left_key = semver_key(left)
    right_key = semver_key(right)
    return (left_key > right_key) - (left_key < right_key)


def read_local_version(skill_path: Path) -> str:
    content = skill_path.read_text()
    metadata_match = re.search(r"^metadata:\s*(\{.*\})\s*$", content, re.MULTILINE)
    if not metadata_match:
        raise SyncError(f"{skill_path} is missing single-line JSON metadata")

    try:
        metadata = json.loads(metadata_match.group(1))
    except json.JSONDecodeError as exc:
        raise SyncError(f"{skill_path} has invalid metadata JSON: {exc}") from exc

    version = metadata.get("openclaw", {}).get("version")
    if not isinstance(version, str) or not version:
        raise SyncError(f"{skill_path} is missing metadata.openclaw.version")
    semver_key(version)
    return version


def should_skip_path(path: Path) -> bool:
    return any(part.startswith(".") for part in path.parts)


def local_file_hashes(skill_dir: Path) -> dict[str, str]:
    if not skill_dir.is_dir():
        raise SyncError(f"skill directory does not exist: {skill_dir}")

    hashes: dict[str, str] = {}
    for path in sorted(skill_dir.rglob("*")):
        rel = path.relative_to(skill_dir)
        if should_skip_path(rel):
            continue
        if not path.is_file():
            continue
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        hashes[rel.as_posix()] = digest

    if "SKILL.md" not in hashes:
        raise SyncError(f"skill directory is missing SKILL.md: {skill_dir}")
    return hashes


def remote_latest(slug: str) -> str:
    data = clawhub_json(["inspect", slug, "--json"])
    version = data.get("latestVersion", {}).get("version")
    if not isinstance(version, str) or not version:
        raise SyncError(f"ClawHub did not return a latest version for {slug}")
    return version


def remote_file_hashes(slug: str, version: str) -> dict[str, str]:
    data = clawhub_json(["inspect", slug, "--version", version, "--files", "--json"])
    selected = data.get("version", {})
    selected_version = selected.get("version")
    if selected_version != version:
        raise SyncError(
            f"ClawHub returned version {selected_version!r}, expected {version!r}"
        )

    files = selected.get("files")
    if not isinstance(files, list):
        raise SyncError(f"ClawHub did not return files for {slug}@{version}")

    hashes: dict[str, str] = {}
    for file in files:
        if not isinstance(file, dict):
            continue
        path = file.get("path")
        sha256 = file.get("sha256")
        if isinstance(path, str) and isinstance(sha256, str):
            hashes[path] = sha256
    return hashes


def format_hash_drift(local: dict[str, str], remote: dict[str, str]) -> str:
    local_paths = set(local)
    remote_paths = set(remote)
    only_local = sorted(local_paths - remote_paths)
    only_remote = sorted(remote_paths - local_paths)
    changed = sorted(path for path in local_paths & remote_paths if local[path] != remote[path])

    lines: list[str] = []
    if changed:
        lines.append("changed: " + ", ".join(changed))
    if only_local:
        lines.append("only local: " + ", ".join(only_local))
    if only_remote:
        lines.append("only remote: " + ", ".join(only_remote))
    return "\n".join(lines) if lines else "unknown hash drift"


def assert_hashes_match(
    *, local: dict[str, str], remote: dict[str, str], version: str
) -> None:
    if local == remote:
        print(f"ClawHub {version} matches local skill files.")
        return

    drift = format_hash_drift(local, remote)
    raise SyncError(
        "local skill files differ from the already-published ClawHub version. "
        "Bump the skill version and add a changelog entry before publishing.\n"
        f"{drift}"
    )


def unreleased_section(changelog: str) -> str:
    match = re.search(
        r"^## Unreleased\s*$([\s\S]*?)(?=^##\s|\Z)", changelog, re.MULTILINE
    )
    if not match:
        raise SyncError("CHANGELOG.md is missing an Unreleased section")
    return match.group(1)


def unreleased_bullets(section: str) -> list[str]:
    bullets: list[str] = []
    current: list[str] = []

    for line in section.splitlines():
        if line.startswith("- "):
            if current:
                bullets.append(" ".join(current))
            current = [line[2:].strip()]
        elif current and (line.startswith("  ") or not line.strip()):
            continuation = line.strip()
            if continuation:
                current.append(continuation)
        elif current:
            bullets.append(" ".join(current))
            current = []

    if current:
        bullets.append(" ".join(current))
    return bullets


def derive_changelog(repo_root: Path) -> str:
    changelog_path = repo_root / "CHANGELOG.md"
    if not changelog_path.is_file():
        raise SyncError("CHANGELOG.md was not found")

    section = unreleased_section(changelog_path.read_text())
    keywords = ("clawhub", "clipboard-memory", "skill")
    matches = [
        bullet.rstrip(".")
        for bullet in unreleased_bullets(section)
        if any(keyword in bullet.lower() for keyword in keywords)
    ]
    if not matches:
        raise SyncError(
            "CHANGELOG.md Unreleased must include a ClawHub, clipboard-memory, "
            "or skill bullet before publishing."
        )
    return " ".join(matches)


def publish_skill(skill_dir: Path, slug: str, version: str, changelog: str) -> None:
    print(f"Publishing {slug}@{version} to ClawHub...")
    result = run_command(
        [
            "clawhub",
            "publish",
            str(skill_dir.resolve()),
            "--slug",
            slug,
            "--version",
            version,
            "--changelog",
            changelog,
        ]
    )
    if result.returncode != 0:
        raise SyncError(result.stdout.strip() or "clawhub publish failed")
    print(result.stdout.strip())


def wait_for_remote_hashes(
    *,
    slug: str,
    version: str,
    timeout_seconds: int,
    poll_interval_seconds: int,
) -> dict[str, str]:
    deadline = time.monotonic() + timeout_seconds
    last_error = ""

    while time.monotonic() < deadline:
        result = run_command(
            ["clawhub", "inspect", slug, "--version", version, "--files", "--json"]
        )
        if result.returncode == 0:
            data = parse_json_from_output(result.stdout)
            selected = data.get("version", {})
            latest = data.get("skill", {}).get("tags", {}).get("latest")
            if selected.get("version") != version:
                last_error = f"selected version was {selected.get('version')!r}"
            elif latest != version:
                last_error = f"latest tag was {latest!r}"
            else:
                files = selected.get("files")
                if isinstance(files, list):
                    return {
                        file["path"]: file["sha256"]
                        for file in files
                        if isinstance(file, dict)
                        and isinstance(file.get("path"), str)
                        and isinstance(file.get("sha256"), str)
                    }
                last_error = "version did not include a files array"
        else:
            last_error = result.stdout.strip()

        print(
            f"Waiting for ClawHub read-back of {slug}@{version}: {last_error}",
            flush=True,
        )
        time.sleep(poll_interval_seconds)

    raise SyncError(
        f"timed out waiting for ClawHub to expose {slug}@{version}: {last_error}"
    )


def check_mode(
    *,
    repo_root: Path,
    local_version: str,
    remote_version: str,
    local_hashes: dict[str, str],
    slug: str,
) -> None:
    ordering = compare_versions(local_version, remote_version)
    if ordering > 0:
        changelog = derive_changelog(repo_root)
        print(
            f"Local {slug}@{local_version} is newer than ClawHub {remote_version}; "
            f"publish will run after merge. Changelog: {changelog}"
        )
        return
    if ordering < 0:
        raise SyncError(
            f"repo skill {slug}@{local_version} is behind ClawHub {remote_version}."
        )

    remote_hashes = remote_file_hashes(slug, remote_version)
    assert_hashes_match(local=local_hashes, remote=remote_hashes, version=local_version)


def publish_mode(
    *,
    repo_root: Path,
    skill_dir: Path,
    slug: str,
    local_version: str,
    remote_version: str,
    local_hashes: dict[str, str],
    timeout_seconds: int,
    poll_interval_seconds: int,
) -> None:
    ordering = compare_versions(local_version, remote_version)
    if ordering < 0:
        raise SyncError(
            f"repo skill {slug}@{local_version} is behind ClawHub {remote_version}."
        )
    if ordering == 0:
        remote_hashes = remote_file_hashes(slug, remote_version)
        assert_hashes_match(local=local_hashes, remote=remote_hashes, version=local_version)
        print(f"No ClawHub publish needed for {slug}@{local_version}.")
        return

    changelog = derive_changelog(repo_root)
    publish_skill(skill_dir, slug, local_version, changelog)
    remote_hashes = wait_for_remote_hashes(
        slug=slug,
        version=local_version,
        timeout_seconds=timeout_seconds,
        poll_interval_seconds=poll_interval_seconds,
    )
    assert_hashes_match(local=local_hashes, remote=remote_hashes, version=local_version)


def main() -> int:
    args = parse_args()
    repo_root = args.repo_root.resolve()
    skill_dir = (repo_root / args.skill_dir).resolve()
    skill_path = skill_dir / "SKILL.md"

    try:
        local_version = read_local_version(skill_path)
        local_hashes = local_file_hashes(skill_dir)
        remote_version = remote_latest(args.slug)
        print(
            f"Local {args.slug}@{local_version}; "
            f"ClawHub latest is {remote_version}."
        )

        if args.mode == "check":
            check_mode(
                repo_root=args.repo_root,
                local_version=local_version,
                remote_version=remote_version,
                local_hashes=local_hashes,
                slug=args.slug,
            )
        else:
            publish_mode(
                repo_root=repo_root,
                skill_dir=skill_dir,
                slug=args.slug,
                local_version=local_version,
                remote_version=remote_version,
                local_hashes=local_hashes,
                timeout_seconds=args.timeout_seconds,
                poll_interval_seconds=args.poll_interval_seconds,
            )
    except SyncError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
