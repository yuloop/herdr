from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

from scripts.herdr_deploy import DeploymentError, deploy


FAKE_HERDR = r'''#!/usr/bin/env python3
import json
import pathlib
import sys

VERSION = __VERSION__
PROTOCOL = __PROTOCOL__
STATE_PATH = pathlib.Path(__STATE_PATH__)


def load():
    return json.loads(STATE_PATH.read_text(encoding="utf-8"))


def save(state):
    STATE_PATH.write_text(json.dumps(state), encoding="utf-8")


args = sys.argv[1:]
if args == ["--version"]:
    print(f"herdr {VERSION}")
elif args == ["status", "client", "--json"]:
    print(json.dumps({"version": VERSION, "protocol": PROTOCOL}))
elif args == ["status", "server", "--json"]:
    state = load()
    print(json.dumps({
        "status": "running",
        "running": True,
        "version": state["server_version"],
        "protocol": state["server_protocol"],
        "compatible": state["server_protocol"] == PROTOCOL,
        "capabilities": {"live_handoff": True, "detached_server_daemon": True},
    }))
elif args == ["workspace", "list"]:
    state = load()
    print(json.dumps({
        "id": "test:workspace:list",
        "result": {
            "type": "workspace_list",
            "workspaces": [
                {"workspace_id": workspace_id}
                for workspace_id in state["workspace_ids"]
            ],
        },
    }))
elif args == ["pane", "list"]:
    state = load()
    print(json.dumps({
        "id": "test:pane:list",
        "result": {
            "type": "pane_list",
            "panes": [{"pane_id": f"p{index}"} for index in range(state["pane_count"])],
        },
    }))
elif args[:2] == ["server", "live-handoff"]:
    state = load()
    expected_version = args[args.index("--expected-version") + 1]
    expected_protocol = int(args[args.index("--expected-protocol") + 1])
    failures = state.get("fail_handoff_versions", [])
    if expected_version in failures:
        failures.remove(expected_version)
        state["fail_handoff_versions"] = failures
        save(state)
        print("simulated handoff failure", file=sys.stderr)
        raise SystemExit(1)
    state["server_version"] = expected_version
    state["server_protocol"] = expected_protocol
    if expected_version == state.get("drop_on_version"):
        state["workspace_ids"] = state["baseline_workspace_ids"][:-1]
        state["pane_count"] = state["baseline_pane_count"] - 1
    else:
        state["workspace_ids"] = list(state["baseline_workspace_ids"])
        state["pane_count"] = state["baseline_pane_count"]
    save(state)
    print("handoff complete", file=sys.stderr)
else:
    print(f"unsupported fake herdr arguments: {args}", file=sys.stderr)
    raise SystemExit(2)
'''


def write_fake_binary(path: Path, version: str, protocol: int, state_path: Path) -> None:
    script = (
        FAKE_HERDR.replace("__VERSION__", repr(version))
        .replace("__PROTOCOL__", repr(protocol))
        .replace("__STATE_PATH__", repr(str(state_path)))
    )
    path.write_text(textwrap.dedent(script), encoding="utf-8", newline="\n")
    path.chmod(0o755)


def reported_version(path: Path) -> str:
    return subprocess.run(
        [path, "--version"], check=True, capture_output=True, text=True
    ).stdout.strip()


@unittest.skipUnless(os.name == "posix", "deployment integration requires POSIX executable semantics")
class HerdrDeployIntegrationTests(unittest.TestCase):
    old_version = "0.8.0-deploy.old"
    new_version = "0.8.0-deploy.new"
    protocol = 19
    source_sha = "c" * 40

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory(prefix="herdr-deploy-test-")
        self.root = Path(self.temp_dir.name)
        self.target = self.root / "herdr"
        self.staged = self.root / "herdr.incoming"
        self.state_path = self.root / "fake-runtime.json"
        self.runtime = {
            "server_version": self.old_version,
            "server_protocol": self.protocol,
            "workspace_ids": ["w1", "w2"],
            "pane_count": 3,
            "baseline_workspace_ids": ["w1", "w2"],
            "baseline_pane_count": 3,
            "fail_handoff_versions": [],
        }
        self.write_runtime()
        write_fake_binary(self.target, self.old_version, self.protocol, self.state_path)
        write_fake_binary(self.staged, self.new_version, self.protocol, self.state_path)

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def write_runtime(self) -> None:
        self.state_path.write_text(json.dumps(self.runtime), encoding="utf-8")

    def deploy_args(self) -> argparse.Namespace:
        artifact_sha = hashlib.sha256(self.staged.read_bytes()).hexdigest()
        return argparse.Namespace(
            target=str(self.target),
            staged=str(self.staged),
            source_sha=self.source_sha,
            expected_version=self.new_version,
            expected_protocol=self.protocol,
            artifact_sha256=artifact_sha,
        )

    def test_successful_handoff_records_source_and_one_backup(self) -> None:
        result = deploy(self.deploy_args())

        self.assertEqual(result["status"], "deployed")
        self.assertEqual(reported_version(self.target), f"herdr {self.new_version}")
        self.assertEqual(
            reported_version(Path(f"{self.target}.previous")),
            f"herdr {self.old_version}",
        )
        self.assertEqual(
            Path(f"{self.target}.source-sha").read_text(encoding="utf-8"),
            f"{self.source_sha}\n",
        )
        self.assertEqual(json.loads(self.state_path.read_text())["server_version"], self.new_version)

    def test_handoff_failure_restores_old_binary_and_server(self) -> None:
        self.runtime["fail_handoff_versions"] = [self.new_version]
        self.write_runtime()

        with self.assertRaisesRegex(DeploymentError, "rollback succeeded"):
            deploy(self.deploy_args())

        self.assertEqual(reported_version(self.target), f"herdr {self.old_version}")
        state = json.loads(self.state_path.read_text())
        self.assertEqual(state["server_version"], self.old_version)
        self.assertEqual(state["workspace_ids"], ["w1", "w2"])
        self.assertFalse(Path(f"{self.target}.source-sha").exists())

    def test_failed_acceptance_uses_reverse_handoff(self) -> None:
        self.runtime["drop_on_version"] = self.new_version
        self.write_runtime()

        with self.assertRaisesRegex(DeploymentError, "reverse live handoff"):
            deploy(self.deploy_args())

        self.assertEqual(reported_version(self.target), f"herdr {self.old_version}")
        state = json.loads(self.state_path.read_text())
        self.assertEqual(state["server_version"], self.old_version)
        self.assertEqual(state["workspace_ids"], ["w1", "w2"])
        self.assertEqual(state["pane_count"], 3)
        self.assertFalse(Path(f"{self.target}.source-sha").exists())


if __name__ == "__main__":
    unittest.main()
