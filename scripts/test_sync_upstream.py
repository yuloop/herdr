from __future__ import annotations

import datetime as dt
import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.sync_upstream import CUSTOM_BRANCH, SyncError, _parser, sync_repository


class SyncUpstreamTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.upstream_bare = self.root / "upstream.git"
        self.upstream_work = self.root / "upstream-work"
        self.local = self.root / "local"

        self.git(self.root, "init", "--bare", "--initial-branch=master", self.upstream_bare)
        self.git(self.root, "init", "--initial-branch=master", self.upstream_work)
        self.configure_identity(self.upstream_work)
        (self.upstream_work / "shared.txt").write_text("base\n", encoding="utf-8")
        self.git(self.upstream_work, "add", "shared.txt")
        self.git(self.upstream_work, "commit", "-m", "upstream base")
        self.git(self.upstream_work, "remote", "add", "origin", self.upstream_bare)
        self.git(self.upstream_work, "push", "-u", "origin", "master")

        self.git(self.root, "clone", self.upstream_bare, self.local)
        self.configure_identity(self.local)
        self.git(self.local, "remote", "rename", "origin", "upstream")
        self.git(self.local, "switch", "-c", CUSTOM_BRANCH)
        (self.local / "fork.txt").write_text("custom\n", encoding="utf-8")
        self.git(self.local, "add", "fork.txt")
        self.git(self.local, "commit", "-m", "fork customization")
        self.custom_head = self.output(self.local, "rev-parse", "HEAD")
        self.original_master = self.output(self.local, "rev-parse", "master")

    def tearDown(self) -> None:
        self.temporary.cleanup()

    @staticmethod
    def git(root: Path, *arguments: object) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", *(str(argument) for argument in arguments)],
            cwd=root,
            check=True,
            capture_output=True,
            text=True,
        )

    @classmethod
    def output(cls, root: Path, *arguments: object) -> str:
        return cls.git(root, *arguments).stdout.strip()

    @classmethod
    def configure_identity(cls, root: Path) -> None:
        cls.git(root, "config", "user.name", "sync-test")
        cls.git(root, "config", "user.email", "sync-test@example.invalid")

    def advance_upstream(self, content: str = "upstream\n") -> str:
        (self.upstream_work / "upstream.txt").write_text(content, encoding="utf-8")
        self.git(self.upstream_work, "add", "upstream.txt")
        self.git(self.upstream_work, "commit", "-m", "upstream change")
        self.git(self.upstream_work, "push", "origin", "master")
        return self.output(self.upstream_work, "rev-parse", "HEAD")

    def sync(self):
        return sync_repository(
            self.local,
            upstream_url=str(self.upstream_bare),
            run_checks=False,
            now=dt.datetime(2026, 8, 7, 4, 5, 6, tzinfo=dt.timezone.utc),
        )

    def test_sync_merges_without_rewriting_custom_history(self) -> None:
        upstream_head = self.advance_upstream()

        result = self.sync()

        self.assertEqual(result.previous_head, self.custom_head)
        self.assertEqual(result.upstream_head, upstream_head)
        self.assertIsNotNone(result.merged_head)
        self.assertEqual(
            result.backup_branch,
            "backup/deploy/zh-with-perf-pre-upstream-20260807-040506",
        )
        self.assertEqual(
            self.output(self.local, "rev-parse", result.backup_branch),
            self.custom_head,
        )
        self.assertEqual(len(self.output(self.local, "rev-list", "--parents", "-n", "1", "HEAD").split()), 3)
        self.assertEqual(self.output(self.local, "rev-parse", "master"), self.original_master)
        self.assertEqual(self.output(self.local, "status", "--porcelain"), "")

        second = self.sync()
        self.assertIsNone(second.merged_head)
        self.assertIsNone(second.backup_branch)

    def test_cli_does_not_offer_a_check_bypass(self) -> None:
        options = {option for action in _parser()._actions for option in action.option_strings}

        self.assertNotIn("--skip-check", options)

    def test_sync_refuses_to_hide_dirty_work(self) -> None:
        (self.local / "unfinished.txt").write_text("keep me\n", encoding="utf-8")

        with self.assertRaisesRegex(SyncError, "working tree is not clean"):
            self.sync()

        self.assertEqual(self.output(self.local, "rev-parse", "HEAD"), self.custom_head)
        self.assertTrue((self.local / "unfinished.txt").is_file())

    def test_sync_aborts_conflict_and_preserves_backup(self) -> None:
        (self.local / "shared.txt").write_text("fork version\n", encoding="utf-8")
        self.git(self.local, "add", "shared.txt")
        self.git(self.local, "commit", "-m", "customize shared file")
        conflict_head = self.output(self.local, "rev-parse", "HEAD")

        (self.upstream_work / "shared.txt").write_text("upstream version\n", encoding="utf-8")
        self.git(self.upstream_work, "add", "shared.txt")
        self.git(self.upstream_work, "commit", "-m", "change shared file upstream")
        self.git(self.upstream_work, "push", "origin", "master")

        with self.assertRaisesRegex(SyncError, "merge conflicted and was aborted"):
            self.sync()

        self.assertEqual(self.output(self.local, "rev-parse", "HEAD"), conflict_head)
        self.assertEqual(self.output(self.local, "status", "--porcelain"), "")
        backup = "backup/deploy/zh-with-perf-pre-upstream-20260807-040506"
        self.assertEqual(self.output(self.local, "rev-parse", backup), conflict_head)
        merge_head = subprocess.run(
            ["git", "rev-parse", "-q", "--verify", "MERGE_HEAD"],
            cwd=self.local,
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(merge_head.returncode, 0)


if __name__ == "__main__":
    unittest.main()
