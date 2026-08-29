from __future__ import annotations

import re
import subprocess
import tempfile
import unittest
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parent.parent
SYNC_WORKFLOW = PROJECT_ROOT / ".github" / "workflows" / "sync-build-deploy.yml"
CI_WORKFLOW = PROJECT_ROOT / ".github" / "workflows" / "ci.yml"
WINDOWS_CHECK = PROJECT_ROOT / "scripts" / "windows_check.ps1"


def job_body(workflow: str, job: str) -> str:
    match = re.search(
        rf"^  {re.escape(job)}:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)",
        workflow,
        flags=re.MULTILINE | re.DOTALL,
    )
    if match is None:
        raise AssertionError(f"workflow job {job!r} is missing")
    return match.group("body")


class CrossPlatformGateTests(unittest.TestCase):
    def test_custom_branch_push_runs_native_ci_matrix(self) -> None:
        workflow = CI_WORKFLOW.read_text(encoding="utf-8")

        self.assertRegex(
            workflow,
            r"branches:\s*\[[^\]]*deploy/zh-with-perf[^\]]*\]",
        )
        self.assertIn("os: ubuntu-latest", workflow)
        self.assertIn("os: windows-latest", workflow)
        self.assertIn(".\\scripts\\windows_check.ps1 -Mode check", workflow)

    def test_windows_local_check_lints_the_linux_release_target(self) -> None:
        script = WINDOWS_CHECK.read_text(encoding="utf-8")

        self.assertGreaterEqual(script.count("x86_64-unknown-linux-musl"), 2)
        linux_gate = script.split("$previousLibghosttyVtSimd", maxsplit=1)[1]
        self.assertIn('"clippy",', linux_gate)
        self.assertIn('"-D",\n        "warnings"', linux_gate)
        self.assertIn("cfg(unix)", script)

    def test_windows_local_check_validates_shared_translation_keys(self) -> None:
        script = WINDOWS_CHECK.read_text(encoding="utf-8")

        self.assertIn('"scripts.test_i18n_key_check"', script)

    def test_windows_local_check_runs_portable_vendor_patch_checks(self) -> None:
        script = WINDOWS_CHECK.read_text(encoding="utf-8")

        self.assertIn('"scripts.test_vendor_libghostty_vt"', script)
        self.assertIn('"scripts.test_vendor_portable_pty"', script)

    def test_windows_local_check_runs_shared_ui_regressions(self) -> None:
        script = WINDOWS_CHECK.read_text(encoding="utf-8")

        self.assertIn('Invoke-CargoTestFilter "ui::"', script)

    def test_promotion_waits_for_linux_and_windows_candidate_gates(self) -> None:
        workflow = SYNC_WORKFLOW.read_text(encoding="utf-8")
        linux = job_body(workflow, "prepare-linux-candidate")
        windows = job_body(workflow, "build-windows")
        promotion = job_body(workflow, "promote-and-deploy")

        self.assertIn("needs: prepare-linux-candidate", windows)
        self.assertIn("needs: [prepare-linux-candidate, build-windows]", promotion)
        self.assertIn("needs.build-windows.result == 'success'", promotion)
        self.assertIn("Run native Windows checks", windows)
        self.assertIn("Build and verify the x86_64 musl artifact", linux)

        push = 'git push origin "HEAD:refs/heads/$CUSTOM_BRANCH"'
        self.assertNotIn(push, linux)
        self.assertNotIn("Deploy Linux with live handoff", linux)
        self.assertIn(push, promotion)
        self.assertIn("Deploy Linux with live handoff", promotion)
        self.assertEqual(workflow.count(push), 1)

    def test_unpushed_candidate_is_verified_by_both_platform_jobs(self) -> None:
        workflow = SYNC_WORKFLOW.read_text(encoding="utf-8")
        linux = job_body(workflow, "prepare-linux-candidate")
        windows = job_body(workflow, "build-windows")
        promotion = job_body(workflow, "promote-and-deploy")

        self.assertIn("git bundle create", linux)
        self.assertIn("candidate.bundle", windows)
        self.assertIn("candidate SHA mismatch", windows)
        self.assertIn("candidate.bundle", promotion)
        self.assertIn('[[ "$(git rev-parse HEAD)" == "$SOURCE_SHA" ]]', promotion)

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
