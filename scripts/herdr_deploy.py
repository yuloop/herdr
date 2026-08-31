#!/usr/bin/env python3
"""Atomically deploy a Herdr binary while preserving live pane processes.

This script is streamed to the deployment host by the fork automation.  It
keeps exactly one previous binary, performs Herdr's live handoff, validates the
server/session after the handoff, and hands the session back to the previous
binary when validation fails.
"""

from __future__ import annotations

import argparse
import dataclasses
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import time
from pathlib import Path, PurePosixPath
from typing import Any, Sequence


SOURCE_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
ARTIFACT_SHA_RE = re.compile(r"^[0-9a-f]{64}$")
VERSION_RE = re.compile(r"^[0-9A-Za-z][0-9A-Za-z._+-]{0,199}$")
SAFE_PATH_RE = re.compile(r"^/[0-9A-Za-z._/-]+$")


class DeploymentError(RuntimeError):
    """A deployment invariant was not satisfied."""


@dataclasses.dataclass(frozen=True)
class ClientIdentity:
    version: str
    protocol: int


@dataclasses.dataclass(frozen=True)
class RuntimeSnapshot:
    version: str
    protocol: int
    compatible: bool
    live_handoff: bool
    workspace_ids: frozenset[str]
    pane_count: int


def _trim_output(value: str, limit: int = 2_000) -> str:
    value = value.strip()
    if len(value) <= limit:
        return value
    return f"...{value[-limit:]}"


def run_command(args: Sequence[str | Path], *, timeout: float = 30.0) -> str:
    rendered_args = [str(arg) for arg in args]
    try:
        result = subprocess.run(
            rendered_args,
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except (OSError, subprocess.TimeoutExpired) as err:
        raise DeploymentError(f"failed to run {rendered_args[0]}: {err}") from err

    if result.returncode != 0:
        detail = _trim_output(result.stderr) or _trim_output(result.stdout)
        suffix = f": {detail}" if detail else ""
        raise DeploymentError(
            f"{rendered_args[0]} exited with status {result.returncode}{suffix}"
        )
    return result.stdout.strip()


def run_json(args: Sequence[str | Path], *, timeout: float = 30.0) -> dict[str, Any]:
    output = run_command(args, timeout=timeout)
    try:
        value = json.loads(output)
    except json.JSONDecodeError as err:
        raise DeploymentError(
            f"{args[0]} returned invalid JSON: {_trim_output(output)}"
        ) from err
    if not isinstance(value, dict):
        raise DeploymentError(f"{args[0]} returned a non-object JSON response")
    return value


def _required_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise DeploymentError(f"{label} is missing or invalid")
    return value


def _required_int(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise DeploymentError(f"{label} is missing or invalid")
    return value


def read_client_identity(binary: Path) -> ClientIdentity:
    status = run_json([binary, "status", "client", "--json"])
    identity = ClientIdentity(
        version=_required_string(status.get("version"), "client version"),
        protocol=_required_int(status.get("protocol"), "client protocol"),
    )
    reported_version = run_command([binary, "--version"])
    if reported_version != f"herdr {identity.version}":
        raise DeploymentError(
            "binary version output does not match its client status: "
            f"{reported_version!r} != {identity.version!r}"
        )
    return identity


def _response_result(response: dict[str, Any], expected_type: str) -> dict[str, Any]:
    result = response.get("result")
    if not isinstance(result, dict) or result.get("type") != expected_type:
        raise DeploymentError(f"expected a {expected_type} response")
    return result


def read_runtime_snapshot(binary: Path) -> RuntimeSnapshot:
    status = run_json([binary, "status", "server", "--json"])
    if status.get("running") is not True or status.get("status") != "running":
        raise DeploymentError("Herdr server is not running")

    capabilities = status.get("capabilities")
    if not isinstance(capabilities, dict):
        raise DeploymentError("server capabilities are missing")

    workspace_result = _response_result(
        run_json([binary, "workspace", "list"]), "workspace_list"
    )
    workspaces = workspace_result.get("workspaces")
    if not isinstance(workspaces, list):
        raise DeploymentError("workspace list is missing")
    workspace_ids: list[str] = []
    for workspace in workspaces:
        if not isinstance(workspace, dict):
            raise DeploymentError("workspace list contains an invalid entry")
        workspace_ids.append(
            _required_string(workspace.get("workspace_id"), "workspace id")
        )
    if len(workspace_ids) != len(set(workspace_ids)):
        raise DeploymentError("workspace list contains duplicate ids")

    pane_result = _response_result(run_json([binary, "pane", "list"]), "pane_list")
    panes = pane_result.get("panes")
    if not isinstance(panes, list):
        raise DeploymentError("pane list is missing")

    return RuntimeSnapshot(
        version=_required_string(status.get("version"), "server version"),
        protocol=_required_int(status.get("protocol"), "server protocol"),
        compatible=status.get("compatible") is True,
        live_handoff=capabilities.get("live_handoff") is True,
        workspace_ids=frozenset(workspace_ids),
        pane_count=len(panes),
    )


def validate_runtime(
    snapshot: RuntimeSnapshot,
    identity: ClientIdentity,
    *,
    before: RuntimeSnapshot | None = None,
) -> None:
    if snapshot.version != identity.version:
        raise DeploymentError(
            f"server version {snapshot.version!r} does not match {identity.version!r}"
        )
    if snapshot.protocol != identity.protocol:
        raise DeploymentError(
            f"server protocol {snapshot.protocol} does not match {identity.protocol}"
        )
    if not snapshot.compatible:
        raise DeploymentError("server reports an incompatible protocol")
    if not snapshot.live_handoff:
        raise DeploymentError("server does not advertise live handoff support")
    if before is None:
        return

    missing_ids = sorted(before.workspace_ids - snapshot.workspace_ids)
    if missing_ids:
        raise DeploymentError(
            "workspace ids disappeared during handoff: " + ", ".join(missing_ids)
        )
    if snapshot.pane_count < before.pane_count:
        raise DeploymentError(
            f"pane count decreased from {before.pane_count} to {snapshot.pane_count}"
        )


def wait_for_runtime(
    binary: Path,
    identity: ClientIdentity,
    before: RuntimeSnapshot,
    *,
    timeout: float,
) -> RuntimeSnapshot:
    deadline = time.monotonic() + timeout
    last_error: Exception | None = None
    while True:
        try:
            snapshot = read_runtime_snapshot(binary)
            validate_runtime(snapshot, identity, before=before)
            return snapshot
        except (DeploymentError, OSError) as err:
            last_error = err
        if time.monotonic() >= deadline:
            raise DeploymentError(
                f"server did not pass health checks within {timeout:.0f}s: {last_error}"
            ) from last_error
        time.sleep(1)


def validate_deploy_path(path: Path, label: str) -> None:
    rendered = path.as_posix()
    posix_path = PurePosixPath(rendered)
    if not rendered.startswith("/") or not SAFE_PATH_RE.fullmatch(rendered):
        raise DeploymentError(f"{label} must be an absolute path without shell metacharacters")
    if rendered == "/" or ".." in posix_path.parts:
        raise DeploymentError(f"unsafe {label}: {rendered}")


def _require_regular_file(path: Path, label: str) -> None:
    try:
        mode = path.lstat().st_mode
    except OSError as err:
        raise DeploymentError(f"cannot inspect {label} at {path}: {err}") from err
    if not stat.S_ISREG(mode) or path.is_symlink():
        raise DeploymentError(f"{label} at {path} is not a regular file")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _fsync_file(path: Path) -> None:
    with path.open("rb") as handle:
        os.fsync(handle.fileno())


def _fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def atomic_copy(source: Path, destination: Path, scratch: Path, name: str) -> None:
    temporary = scratch / name
    shutil.copy2(source, temporary)
    _fsync_file(temporary)
    os.replace(temporary, destination)
    _fsync_directory(destination.parent)


def atomic_write_text(destination: Path, value: str, scratch: Path, name: str) -> None:
    temporary = scratch / name
    with temporary.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(value)
        handle.flush()
        os.fsync(handle.fileno())
    os.chmod(temporary, 0o644)
    os.replace(temporary, destination)
    _fsync_directory(destination.parent)


def perform_handoff(binary: Path, identity: ClientIdentity) -> None:
    run_command(
        [
            binary,
            "server",
            "live-handoff",
            "--import-exe",
            binary,
            "--expected-protocol",
            str(identity.protocol),
            "--expected-version",
            identity.version,
        ],
        timeout=75,
    )


def rollback_to_old(
    *,
    target: Path,
    backup: Path,
    new_recovery_binary: Path,
    old_identity: ClientIdentity,
    new_identity: ClientIdentity,
    before: RuntimeSnapshot,
    scratch: Path,
) -> str:
    atomic_copy(backup, target, scratch, "restore-old.next")
    os.chmod(target, 0o755)

    try:
        wait_for_runtime(target, old_identity, before, timeout=5)
        return "old binary restored; original server remained healthy"
    except DeploymentError as passive_error:
        try:
            perform_handoff(target, old_identity)
            wait_for_runtime(target, old_identity, before, timeout=30)
            return "old binary and old server restored by reverse live handoff"
        except DeploymentError as reverse_error:
            try:
                wait_for_runtime(new_recovery_binary, new_identity, before, timeout=5)
            except DeploymentError as new_error:
                raise DeploymentError(
                    "rollback could not verify either server; "
                    f"old check: {passive_error}; reverse handoff: {reverse_error}; "
                    f"new check: {new_error}"
                ) from reverse_error

            atomic_copy(
                new_recovery_binary,
                target,
                scratch,
                "restore-new-after-failed-rollback.next",
            )
            os.chmod(target, 0o755)
            raise DeploymentError(
                "reverse handoff failed; the healthy new server and matching binary were retained: "
                f"{reverse_error}"
            ) from reverse_error


def deploy(args: argparse.Namespace) -> dict[str, Any]:
    target = Path(args.target)
    staged = Path(args.staged)
    locales_path: Path | None = Path(args.locales_tarball) if args.locales_tarball else None
    validate_deploy_path(target, "target")
    validate_deploy_path(staged, "staged binary")
    if target.parent.resolve() != staged.parent.resolve():
        raise DeploymentError("staged binary must be on the target filesystem and directory")
    if not SOURCE_SHA_RE.fullmatch(args.source_sha):
        raise DeploymentError("source SHA must contain 40 lowercase hexadecimal characters")
    if not ARTIFACT_SHA_RE.fullmatch(args.artifact_sha256):
        raise DeploymentError("artifact SHA-256 must contain 64 lowercase hexadecimal characters")
    if not VERSION_RE.fullmatch(args.expected_version):
        raise DeploymentError("expected version has an unsafe or invalid format")
    if args.expected_protocol < 0:
        raise DeploymentError("expected protocol must be non-negative")

    if locales_path is not None:
        if sha256_file(locales_path) != args.locales_sha256:
            raise DeploymentError("locales tarball SHA-256 does not match the build artifact")
        _require_regular_file(locales_path, "locales tarball")

    _require_regular_file(target, "installed binary")
    _require_regular_file(staged, "staged binary")
    if sha256_file(staged) != args.artifact_sha256:
        raise DeploymentError("staged binary SHA-256 does not match the build artifact")

    os.chmod(staged, 0o755)
    new_identity = read_client_identity(staged)
    expected_identity = ClientIdentity(args.expected_version, args.expected_protocol)
    if new_identity != expected_identity:
        raise DeploymentError(
            f"staged identity {new_identity} does not match expected {expected_identity}"
        )

    old_identity = read_client_identity(target)
    before = read_runtime_snapshot(target)
    validate_runtime(before, old_identity)

    backup = Path(f"{target}.previous")
    state_file = Path(f"{target}.source-sha")
    scratch = Path(tempfile.mkdtemp(prefix=".herdr-deploy-", dir=target.parent))
    new_recovery_binary = scratch / "new-binary"
    installed = False
    completed = False

    try:
        atomic_copy(target, backup, scratch, "previous.next")
        shutil.copy2(staged, new_recovery_binary)
        os.chmod(new_recovery_binary, 0o755)
        _fsync_file(new_recovery_binary)

        _fsync_file(staged)
        os.replace(staged, target)
        _fsync_directory(target.parent)
        installed = True

        perform_handoff(target, new_identity)
        after = wait_for_runtime(target, new_identity, before, timeout=30)

        if locales_path is not None:
            locales_dest = target.parent / "locales"
            os.makedirs(locales_dest, exist_ok=True)
            install_args = [
                "tar", "-xzf", str(locales_path),
                "-C", str(locales_dest),
            ]
            result = subprocess.run(
                install_args,
                check=False,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if result.returncode != 0:
                raise DeploymentError(
                    "failed to extract locales tarball: "
                    f"{_trim_output(result.stderr or result.stdout)}"
                )

        atomic_write_text(
            state_file,
            f"{args.source_sha}\n",
            scratch,
            "source-sha.next",
        )
        completed = True
        return {
            "status": "deployed",
            "source_sha": args.source_sha,
            "artifact_sha256": args.artifact_sha256,
            "version": new_identity.version,
            "protocol": new_identity.protocol,
            "workspace_ids": sorted(after.workspace_ids),
            "pane_count_before": before.pane_count,
            "pane_count_after": after.pane_count,
            "backup": str(backup),
            "state_file": str(state_file),
        }
    except Exception as deploy_error:
        if not installed:
            raise
        try:
            rollback_result = rollback_to_old(
                target=target,
                backup=backup,
                new_recovery_binary=new_recovery_binary,
                old_identity=old_identity,
                new_identity=new_identity,
                before=before,
                scratch=scratch,
            )
        except Exception as rollback_error:
            raise DeploymentError(
                f"deployment failed: {deploy_error}; rollback failed: {rollback_error}"
            ) from rollback_error
        raise DeploymentError(
            f"deployment failed: {deploy_error}; rollback succeeded: {rollback_result}"
        ) from deploy_error
    finally:
        if staged.exists():
            staged.unlink()
        shutil.rmtree(scratch, ignore_errors=True)
        if completed:
            _fsync_directory(target.parent)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", help="absolute installed Herdr binary path")
    parser.add_argument("staged", help="absolute staged binary path in the same directory")
    parser.add_argument("locales_tarball", help="absolute path to staged locales tar.gz, or empty string")
    parser.add_argument("locales_sha256", help="expected established locales SHA-256 (64-char lowercase hex)")
    parser.add_argument("source_sha", help="40-character source commit SHA")
    parser.add_argument("expected_version", help="version reported by the staged binary")
    parser.add_argument("expected_protocol", type=int, help="protocol reported by the staged binary")
    parser.add_argument("artifact_sha256", help="expected staged binary SHA-256")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    try:
        result = deploy(parse_args(argv))
    except (DeploymentError, OSError) as err:
        print(f"herdr deployment error: {err}", file=sys.stderr)
        return 1
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
