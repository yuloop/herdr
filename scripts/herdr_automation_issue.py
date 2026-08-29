#!/usr/bin/env python3
"""Maintain the single failure issue for fork sync/build/deploy automation."""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import hashlib
import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any, Protocol, Sequence


ISSUE_TITLE = "[automation] Upstream sync/build/deploy failed"
FINGERPRINT_MARKER_RE = re.compile(
    r"<!-- herdr-automation-fingerprint:([0-9a-f]{64}) -->"
)
SHA_RE = re.compile(r"^[0-9a-f]{40}$")


class IssueError(RuntimeError):
    """GitHub issue reporting failed."""


@dataclasses.dataclass(frozen=True)
class Failure:
    stage: str
    upstream_sha: str
    custom_sha: str
    conflicts: tuple[str, ...]
    summary: str
    run_url: str


class IssueClient(Protocol):
    def find_issue(self) -> dict[str, Any] | None: ...

    def create_issue(self, body: str) -> dict[str, Any]: ...

    def update_issue(self, number: int, body: str, *, state: str) -> None: ...

    def comment(self, number: int, body: str) -> None: ...


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def _normalized(value: str, fallback: str = "unknown") -> str:
    value = value.strip()
    return value if value else fallback


def _normalized_sha(value: str) -> str:
    value = value.strip().lower()
    return value if SHA_RE.fullmatch(value) else "unknown"


def failure_fingerprint(failure: Failure) -> str:
    stable = {
        "stage": _normalized(failure.stage),
        "upstream_sha": _normalized_sha(failure.upstream_sha),
        "custom_sha": _normalized_sha(failure.custom_sha),
        "conflicts": sorted({_normalized(item) for item in failure.conflicts if item.strip()}),
        "summary": _normalized(failure.summary),
    }
    encoded = json.dumps(stable, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def extract_fingerprint(body: str) -> str | None:
    match = FINGERPRINT_MARKER_RE.search(body)
    return match.group(1) if match else None


def _inline_code(value: str) -> str:
    sanitized = _normalized(value).replace("`", "ˋ")
    return f"`{sanitized}`"


def _safe_link(url: str) -> str:
    url = url.strip()
    if url.startswith("https://github.com/") or url.startswith("https://"):
        return f"[Open workflow run]({url})"
    return "Workflow link unavailable"


def render_failure_body(failure: Failure, *, seen_at: str) -> str:
    fingerprint = failure_fingerprint(failure)
    conflict_lines = [item.strip() for item in failure.conflicts if item.strip()]
    conflicts = (
        "\n".join(f"- `{item.replace('`', '')}`" for item in sorted(set(conflict_lines)))
        if conflict_lines
        else "_None reported._"
    )
    summary = _normalized(failure.summary)[:4_000]
    return "\n".join(
        [
            "The fork-only hourly automation is currently failing. No issue or pull request was sent to the upstream repository.",
            "",
            f"- Stage: {_inline_code(failure.stage)}",
            f"- Upstream SHA: {_inline_code(_normalized_sha(failure.upstream_sha))}",
            f"- Custom SHA: {_inline_code(_normalized_sha(failure.custom_sha))}",
            f"- Last seen: `{seen_at}`",
            f"- Run: {_safe_link(failure.run_url)}",
            "",
            "### Conflicting files",
            "",
            conflicts,
            "",
            "### Failure summary",
            "",
            summary,
            "",
            "This body is updated in place for repeated occurrences of the same failure.",
            f"<!-- herdr-automation-fingerprint:{fingerprint} -->",
        ]
    )


def render_failure_comment(failure: Failure, *, seen_at: str) -> str:
    return "\n".join(
        [
            f"Failure changed or recurred at `{seen_at}`.",
            "",
            f"- Stage: {_inline_code(failure.stage)}",
            f"- Upstream SHA: {_inline_code(_normalized_sha(failure.upstream_sha))}",
            f"- Custom SHA: {_inline_code(_normalized_sha(failure.custom_sha))}",
            f"- Run: {_safe_link(failure.run_url)}",
        ]
    )


def report_failure(client: IssueClient, failure: Failure, *, seen_at: str | None = None) -> None:
    seen_at = seen_at or utc_now()
    body = render_failure_body(failure, seen_at=seen_at)
    fingerprint = failure_fingerprint(failure)
    issue = client.find_issue()
    if issue is None:
        client.create_issue(body)
        return

    number = issue.get("number")
    if isinstance(number, bool) or not isinstance(number, int):
        raise IssueError("existing automation issue has no valid number")
    old_body = issue.get("body") if isinstance(issue.get("body"), str) else ""
    old_fingerprint = extract_fingerprint(old_body)
    was_closed = issue.get("state") == "closed"
    if old_fingerprint != fingerprint or was_closed:
        client.comment(number, render_failure_comment(failure, seen_at=seen_at))
    client.update_issue(number, body, state="open")


def report_recovery(
    client: IssueClient,
    *,
    custom_sha: str,
    upstream_sha: str,
    run_url: str,
    recovered_at: str | None = None,
) -> None:
    issue = client.find_issue()
    if issue is None or issue.get("state") != "open":
        return
    number = issue.get("number")
    if isinstance(number, bool) or not isinstance(number, int):
        raise IssueError("existing automation issue has no valid number")
    recovered_at = recovered_at or utc_now()
    comment = "\n".join(
        [
            f"Automation recovered at `{recovered_at}` and the deployed source is current.",
            "",
            f"- Upstream SHA: {_inline_code(_normalized_sha(upstream_sha))}",
            f"- Custom SHA: {_inline_code(_normalized_sha(custom_sha))}",
            f"- Run: {_safe_link(run_url)}",
        ]
    )
    client.comment(number, comment)
    existing_body = issue.get("body") if isinstance(issue.get("body"), str) else ""
    client.update_issue(number, existing_body, state="closed")


class GitHubIssueClient:
    def __init__(self, repository: str, token: str, api_url: str) -> None:
        if repository.count("/") != 1:
            raise IssueError("repository must use owner/name format")
        if not token:
            raise IssueError("GITHUB_TOKEN is required")
        self.repository = repository
        self.token = token
        self.api_url = api_url.rstrip("/")

    def _request(
        self, method: str, path: str, payload: dict[str, Any] | None = None
    ) -> Any:
        data = None if payload is None else json.dumps(payload).encode()
        request = urllib.request.Request(
            f"{self.api_url}{path}",
            data=data,
            method=method,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self.token}",
                "Content-Type": "application/json",
                "User-Agent": "herdr-fork-automation",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                raw = response.read()
        except urllib.error.HTTPError as err:
            detail = err.read().decode("utf-8", errors="replace")[:1_000]
            raise IssueError(f"GitHub API {method} {path} failed ({err.code}): {detail}") from err
        except urllib.error.URLError as err:
            raise IssueError(f"GitHub API {method} {path} failed: {err}") from err
        if not raw:
            return None
        try:
            return json.loads(raw)
        except json.JSONDecodeError as err:
            raise IssueError("GitHub API returned invalid JSON") from err

    def find_issue(self) -> dict[str, Any] | None:
        page = 1
        while True:
            query = urllib.parse.urlencode(
                {
                    "state": "all",
                    "per_page": 100,
                    "page": page,
                    "sort": "created",
                    "direction": "asc",
                }
            )
            response = self._request(
                "GET", f"/repos/{self.repository}/issues?{query}"
            )
            if not isinstance(response, list):
                raise IssueError("GitHub issue listing returned an invalid response")
            for item in response:
                if (
                    isinstance(item, dict)
                    and "pull_request" not in item
                    and item.get("title") == ISSUE_TITLE
                ):
                    return item
            if len(response) < 100:
                return None
            page += 1

    def create_issue(self, body: str) -> dict[str, Any]:
        response = self._request(
            "POST",
            f"/repos/{self.repository}/issues",
            {"title": ISSUE_TITLE, "body": body},
        )
        if not isinstance(response, dict):
            raise IssueError("GitHub issue creation returned an invalid response")
        return response

    def update_issue(self, number: int, body: str, *, state: str) -> None:
        payload: dict[str, Any] = {"body": body, "state": state}
        if state == "closed":
            payload["state_reason"] = "completed"
        self._request("PATCH", f"/repos/{self.repository}/issues/{number}", payload)

    def comment(self, number: int, body: str) -> None:
        self._request(
            "POST",
            f"/repos/{self.repository}/issues/{number}/comments",
            {"body": body},
        )


def _read_conflicts(path: str | None) -> tuple[str, ...]:
    if not path:
        return ()
    conflict_path = Path(path)
    if not conflict_path.exists():
        return ()
    return tuple(line.strip() for line in conflict_path.read_text(encoding="utf-8").splitlines() if line.strip())


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", default=os.environ.get("GITHUB_REPOSITORY", ""))
    parser.add_argument("--api-url", default=os.environ.get("GITHUB_API_URL", "https://api.github.com"))
    subparsers = parser.add_subparsers(dest="command", required=True)

    failure = subparsers.add_parser("failure")
    failure.add_argument("--stage", required=True)
    failure.add_argument("--upstream-sha", default="unknown")
    failure.add_argument("--custom-sha", default="unknown")
    failure.add_argument("--conflicts-file")
    failure.add_argument("--summary", required=True)
    failure.add_argument("--run-url", required=True)

    recovery = subparsers.add_parser("recover")
    recovery.add_argument("--upstream-sha", default="unknown")
    recovery.add_argument("--custom-sha", default="unknown")
    recovery.add_argument("--run-url", required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        client = GitHubIssueClient(
            args.repository,
            os.environ.get("GITHUB_TOKEN", ""),
            args.api_url,
        )
        if args.command == "failure":
            report_failure(
                client,
                Failure(
                    stage=args.stage,
                    upstream_sha=args.upstream_sha,
                    custom_sha=args.custom_sha,
                    conflicts=_read_conflicts(args.conflicts_file),
                    summary=args.summary,
                    run_url=args.run_url,
                ),
            )
        else:
            report_recovery(
                client,
                custom_sha=args.custom_sha,
                upstream_sha=args.upstream_sha,
                run_url=args.run_url,
            )
    except (IssueError, OSError) as err:
        err_msg = str(err)
        if "(410)" in err_msg or "Issues has been disabled" in err_msg:
            print("automation issue: issues are disabled in this repository; skipping", file=sys.stderr)
            return 0
        print(f"automation issue error: {err}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
