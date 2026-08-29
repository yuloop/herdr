from __future__ import annotations

import unittest
from pathlib import Path

from scripts.herdr_deploy import (
    ClientIdentity,
    DeploymentError,
    RuntimeSnapshot,
    validate_deploy_path,
    validate_runtime,
)


def snapshot(
    *,
    version: str = "0.8.0-deploy.test",
    protocol: int = 19,
    workspaces: frozenset[str] = frozenset({"w1", "w2"}),
    panes: int = 3,
) -> RuntimeSnapshot:
    return RuntimeSnapshot(
        version=version,
        protocol=protocol,
        compatible=True,
        live_handoff=True,
        workspace_ids=workspaces,
        pane_count=panes,
    )


class HerdrDeployTests(unittest.TestCase):
    def test_runtime_accepts_preserved_workspaces_and_additional_panes(self) -> None:
        before = snapshot()
        after = snapshot(
            workspaces=frozenset({"w1", "w2", "w3"}),
            panes=4,
        )

        validate_runtime(after, ClientIdentity(after.version, after.protocol), before=before)

    def test_runtime_rejects_a_missing_workspace(self) -> None:
        before = snapshot()
        after = snapshot(workspaces=frozenset({"w1"}))

        with self.assertRaisesRegex(DeploymentError, "workspace ids disappeared"):
            validate_runtime(after, ClientIdentity(after.version, after.protocol), before=before)

    def test_runtime_rejects_a_lower_pane_count(self) -> None:
        before = snapshot()
        after = snapshot(panes=2)

        with self.assertRaisesRegex(DeploymentError, "pane count decreased"):
            validate_runtime(after, ClientIdentity(after.version, after.protocol), before=before)

    def test_runtime_rejects_protocol_mismatch(self) -> None:
        current = snapshot()

        with self.assertRaisesRegex(DeploymentError, "server protocol"):
            validate_runtime(current, ClientIdentity(current.version, 20))

    def test_deploy_path_requires_a_safe_absolute_path(self) -> None:
        validate_deploy_path(Path("/root/.local/bin/herdr"), "target")

        for unsafe in ("relative/herdr", "/", "/root/bin/herdr;rm", "/root/../herdr"):
            with self.subTest(unsafe=unsafe), self.assertRaises(DeploymentError):
                validate_deploy_path(Path(unsafe), "target")


if __name__ == "__main__":
    unittest.main()
