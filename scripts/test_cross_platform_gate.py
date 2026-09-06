from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parent.parent


class CrossPlatformGateTests(unittest.TestCase):
    """Verify the candidate bundle handoff contract used by fork automation.

    The sync-build-deploy workflow was removed with the 31.4 deploy target,
    but the bundle round-trip contract is still what future automation relies
    on, so it is kept as a pure git-level test.
    """

    def test_candidate_bundle_round_trip_needs_only_the_custom_base(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            recipient = root / "recipient"
            bundle = root / "candidate.bundle"
            self.git(root, "init", "--initial-branch=deploy/zh-with-perf", source)
            self.configure_identity(source)

            (source / "base.txt").write_text("base\n", encoding="utf-8")
            self.commit_all(source, "base")
            base = self.output(source, "rev-parse", "HEAD")

            (source / "custom.txt").write_text("custom\n", encoding="utf-8")
            self.commit_all(source, "custom patch")
            custom_base = self.output(source, "rev-parse", "HEAD")
            self.git(root, "clone", source, recipient)

            self.git(source, "switch", "-c", "upstream", base)
            (source / "upstream.txt").write_text("upstream\n", encoding="utf-8")
            self.commit_all(source, "upstream change")
            self.git(source, "switch", "deploy/zh-with-perf")
            self.git(source, "merge", "--no-ff", "upstream", "-m", "merge candidate")
            candidate = self.output(source, "rev-parse", "HEAD")
            self.git(
                source,
                "branch",
                "--force",
                "automation/candidate-export",
                candidate,
            )
            self.git(
                source,
                "bundle",
                "create",
                bundle,
                "refs/heads/automation/candidate-export",
                f"^{custom_base}",
            )

            self.git(recipient, "bundle", "verify", bundle)
            self.git(
                recipient,
                "fetch",
                bundle,
                "refs/heads/automation/candidate-export:refs/remotes/candidate/export",
            )
            self.git(recipient, "switch", "--detach", candidate)

            self.assertEqual(self.output(recipient, "rev-parse", "HEAD"), candidate)
            self.assertEqual((recipient / "custom.txt").read_text(encoding="utf-8"), "custom\n")
            self.assertEqual(
                (recipient / "upstream.txt").read_text(encoding="utf-8"),
                "upstream\n",
            )

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
        cls.git(root, "config", "user.name", "cross-platform-gate-test")
        cls.git(root, "config", "user.email", "gate-test@example.invalid")

    @classmethod
    def commit_all(cls, root: Path, message: str) -> None:
        cls.git(root, "add", ".")
        cls.git(root, "commit", "-m", message)


if __name__ == "__main__":
    unittest.main()
