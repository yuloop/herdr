#!/usr/bin/env python3
"""Safely merge the official Herdr branch into the fork integration branch."""

from __future__ import annotations

import argparse
import datetime as dt
import os
import platform
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parent.parent
OFFICIAL_UPSTREAM_URL = "https://github.com/herdrdev/herdr.git"
UPSTREAM_REMOTE = "upstream"
UPSTREAM_BRANCH = "master"
CUSTOM_BRANCH = "deploy/zh-with-perf"
MERGE_MESSAGE = "chore: merge current upstream into local integration branch"


class SyncError(RuntimeError):
    """Raised when the repository cannot be synchronized safely."""


@dataclass(frozen=True)
class SyncResult:
    previous_head: str
    upstream_head: str
    merged_head: str | None
    backup_branch: str | None


def _run(
    command: list[str],
    *,
    root: Path,
    check: bool = True,
    capture: bool = True,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        command,
        cwd=root,
        check=False,
        text=True,
        capture_output=capture,
    )
    if check and result.returncode != 0:
        detail = (result.stderr or result.stdout or "command failed").strip()
        raise SyncError(f"{' '.join(command)}: {detail}")
    return result


def _git(
    *arguments: str,
    root: Path,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    return _run(["git", *arguments], root=root, check=check)


def _git_output(*arguments: str, root: Path) -> str:
    return _git(*arguments, root=root).stdout.strip()


def _same_path(left: Path, right: Path) -> bool:
    return os.path.normcase(str(left.resolve())) == os.path.normcase(str(right.resolve()))


def _normalized_remote_url(url: str) -> str:
    normalized = url.strip().rstrip("/")
    if normalized.lower().endswith(".git"):
        normalized = normalized[:-4]
    return normalized.lower()


def _merge_in_progress(root: Path) -> bool:
    return _git("rev-parse", "-q", "--verify", "MERGE_HEAD", root=root, check=False).returncode == 0


def _is_ancestor(ancestor: str, descendant: str, *, root: Path) -> bool:
    return (
        _git(
            "merge-base",
            "--is-ancestor",
            ancestor,
            descendant,
            root=root,
            check=False,
        ).returncode
        == 0
    )


def _ensure_upstream_remote(
    *,
    root: Path,
    expected_url: str,
    fix_remote: bool,
) -> None:
    current = _git(
        "remote",
        "get-url",
        UPSTREAM_REMOTE,
        root=root,
        check=False,
    )
    if current.returncode != 0:
        _git("remote", "add", UPSTREAM_REMOTE, expected_url, root=root)
    elif _normalized_remote_url(current.stdout) != _normalized_remote_url(expected_url):
        if not fix_remote:
            raise SyncError(
                f"remote '{UPSTREAM_REMOTE}' is {current.stdout.strip()!r}, expected "
                f"{expected_url!r}; rerun with --fix-remote after reviewing the mismatch"
            )
        _git("remote", "set-url", UPSTREAM_REMOTE, expected_url, root=root)

    # The official remote is fetch-only. This prevents an accidental push even
    # when a credentialed GitHub account is active on the machine.
    _git("remote", "set-url", "--push", UPSTREAM_REMOTE, "DISABLED", root=root)


def _backup_branch_name(now: dt.datetime) -> str:
    timestamp = now.astimezone(dt.timezone.utc).strftime("%Y%m%d-%H%M%S")
    return f"backup/{CUSTOM_BRANCH}-pre-upstream-{timestamp}"


def _abort_merge(root: Path) -> None:
    if _merge_in_progress(root):
        _git("merge", "--abort", root=root, check=False)


def sync_repository(
    root: Path,
    *,
    upstream_url: str = OFFICIAL_UPSTREAM_URL,
    fix_remote: bool = False,
    run_checks: bool = True,
    now: dt.datetime | None = None,
) -> SyncResult:
    """Fetch and verify an upstream merge without rewriting fork commits."""

    root = root.resolve()
    discovered_root = Path(_git_output("rev-parse", "--show-toplevel", root=root))
    if not _same_path(root, discovered_root):
        raise SyncError(f"run from repository root {discovered_root}")

    branch = _git_output("branch", "--show-current", root=root)
    if branch != CUSTOM_BRANCH:
        raise SyncError(f"current branch is {branch!r}; expected {CUSTOM_BRANCH!r}")
    if _merge_in_progress(root):
        raise SyncError("a merge is already in progress")

    dirty = _git_output("status", "--porcelain=v1", "--untracked-files=all", root=root)
    if dirty:
        raise SyncError(
            "working tree is not clean; commit the functional patch partitions first:\n"
            + dirty
        )

    _ensure_upstream_remote(
        root=root,
        expected_url=upstream_url,
        fix_remote=fix_remote,
    )
    _git(
        "fetch",
        "--no-tags",
        UPSTREAM_REMOTE,
        f"+refs/heads/{UPSTREAM_BRANCH}:refs/remotes/{UPSTREAM_REMOTE}/{UPSTREAM_BRANCH}",
        root=root,
    )

    upstream_ref = f"refs/remotes/{UPSTREAM_REMOTE}/{UPSTREAM_BRANCH}"
    previous_head = _git_output("rev-parse", "HEAD", root=root)
    upstream_head = _git_output("rev-parse", upstream_ref, root=root)
    if _is_ancestor(upstream_head, previous_head, root=root):
        return SyncResult(previous_head, upstream_head, None, None)

    backup_branch = _backup_branch_name(now or dt.datetime.now(dt.timezone.utc))
    _git("branch", backup_branch, previous_head, root=root)

    merge = _git("merge", "--no-ff", "--no-commit", upstream_ref, root=root, check=False)
    if merge.returncode != 0:
        conflicts = _git(
            "diff",
            "--name-only",
            "--diff-filter=U",
            root=root,
            check=False,
        ).stdout.strip()
        _abort_merge(root)
        detail = f"\nconflicts:\n{conflicts}" if conflicts else ""
        raise SyncError(f"upstream merge conflicted and was aborted{detail}")

    try:
        diff_check = _git("diff", "--check", "HEAD", root=root, check=False)
        if diff_check.returncode != 0:
            raise SyncError(f"merged tree failed git diff --check:\n{diff_check.stdout}")

        if run_checks:
            checked = _run(["just", "check"], root=root, check=False, capture=False)
            if checked.returncode != 0:
                raise SyncError(f"merged tree failed just check (exit {checked.returncode})")

        _git("commit", "-m", MERGE_MESSAGE, root=root)
        merged_head = _git_output("rev-parse", "HEAD", root=root)
        return SyncResult(previous_head, upstream_head, merged_head, backup_branch)
    except BaseException:
        _abort_merge(root)
        raise


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "merge herdrdev/herdr:master into deploy/zh-with-perf after validation; "
            "never rebase, stash, deploy, or push"
        )
    )
    parser.add_argument(
        "--fix-remote",
        action="store_true",
        help="replace a mismatched upstream fetch URL with the official repository",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        result = sync_repository(
            PROJECT_ROOT,
            fix_remote=args.fix_remote,
        )
    except (SyncError, KeyboardInterrupt) as error:
        print(f"upstream sync failed: {error}", file=sys.stderr)
        return 1

    if result.merged_head is None:
        print(f"already contains upstream {result.upstream_head}")
    else:
        print(f"merged upstream {result.upstream_head}")
        print(f"new integration commit: {result.merged_head}")
        print(f"backup branch: {result.backup_branch}")
        print(f"local validation host: {platform.system() or 'unknown'}")
        print("native Linux and Windows CI must both pass before promotion or deployment")
        print("no branch was pushed or deployed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
