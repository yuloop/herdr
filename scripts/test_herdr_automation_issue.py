from __future__ import annotations

import unittest
from typing import Any

from scripts.herdr_automation_issue import (
    Failure,
    extract_fingerprint,
    failure_fingerprint,
    render_failure_body,
    report_failure,
    report_recovery,
)


class FakeIssueClient:
    def __init__(self, issue: dict[str, Any] | None = None) -> None:
        self.issue = issue
        self.created: list[str] = []
        self.updated: list[tuple[int, str, str]] = []
        self.comments: list[tuple[int, str]] = []

    def find_issue(self) -> dict[str, Any] | None:
        return self.issue

    def create_issue(self, body: str) -> dict[str, Any]:
        self.created.append(body)
        return {"number": 1, "body": body, "state": "open"}

    def update_issue(self, number: int, body: str, *, state: str) -> None:
        self.updated.append((number, body, state))

    def comment(self, number: int, body: str) -> None:
        self.comments.append((number, body))


def failure(summary: str = "cargo tests failed") -> Failure:
    return Failure(
        stage="tests",
        upstream_sha="a" * 40,
        custom_sha="b" * 40,
        conflicts=("src/a.rs",),
        summary=summary,
        run_url="https://github.com/example/herdr/actions/runs/1",
    )


class AutomationIssueTests(unittest.TestCase):
    def test_failure_body_contains_a_stable_fingerprint(self) -> None:
        current = failure()
        body = render_failure_body(current, seen_at="2026-08-04T00:00:00+00:00")

        self.assertEqual(extract_fingerprint(body), failure_fingerprint(current))
        self.assertIn("src/a.rs", body)
        self.assertIn("cargo tests failed", body)

    def test_new_failure_creates_the_unique_issue(self) -> None:
        client = FakeIssueClient()

        report_failure(client, failure(), seen_at="2026-08-04T00:00:00+00:00")

        self.assertEqual(len(client.created), 1)
        self.assertEqual(client.updated, [])
        self.assertEqual(client.comments, [])

    def test_same_open_failure_only_updates_status(self) -> None:
        current = failure()
        old_body = render_failure_body(current, seen_at="2026-08-03T00:00:00+00:00")
        client = FakeIssueClient({"number": 7, "body": old_body, "state": "open"})

        report_failure(client, current, seen_at="2026-08-04T00:00:00+00:00")

        self.assertEqual(len(client.updated), 1)
        self.assertEqual(client.updated[0][2], "open")
        self.assertEqual(client.comments, [])

    def test_changed_failure_adds_a_comment(self) -> None:
        old_body = render_failure_body(failure(), seen_at="2026-08-03T00:00:00+00:00")
        client = FakeIssueClient({"number": 7, "body": old_body, "state": "open"})

        report_failure(
            client,
            failure("SSH deployment failed"),
            seen_at="2026-08-04T00:00:00+00:00",
        )

        self.assertEqual(len(client.comments), 1)
        self.assertEqual(client.updated[0][2], "open")

    def test_recovery_comments_and_closes_an_open_issue(self) -> None:
        client = FakeIssueClient({"number": 7, "body": "failure", "state": "open"})

        report_recovery(
            client,
            custom_sha="b" * 40,
            upstream_sha="a" * 40,
            run_url="https://github.com/example/herdr/actions/runs/2",
            recovered_at="2026-08-04T01:00:00+00:00",
        )

        self.assertEqual(len(client.comments), 1)
        self.assertEqual(client.updated, [(7, "failure", "closed")])


if __name__ == "__main__":
    unittest.main()
