#!/bin/sh
# installed by herdr
# managed by herdr; reinstalling or updating the integration overwrites this file.
# add custom hooks beside this file instead of editing it.
# HERDR_INTEGRATION_ID=qwen
# HERDR_INTEGRATION_VERSION=1

action="${1:-}"
case "$action" in
  session|working|blocked|idle|release) ;;
  *) exit 0 ;;
esac

[ "${HERDR_ENV:-}" = "1" ] || exit 0
[ -n "${HERDR_SOCKET_PATH:-}" ] || exit 0
[ -n "${HERDR_PANE_ID:-}" ] || exit 0
command -v python3 >/dev/null 2>&1 || exit 0

python3 -c '
import json
import os
import socket
import sys
import time

action = sys.argv[1]
try:
    payload = json.load(sys.stdin)
except Exception:
    payload = {}

if payload.get("agent_id"):
    raise SystemExit(0)

session_id = payload.get("session_id")
if not isinstance(session_id, str) or not session_id:
    session_id = None
session_start_source = payload.get("source")
if not isinstance(session_start_source, str) or not session_start_source:
    session_start_source = None

pane_id = os.environ["HERDR_PANE_ID"]
source = "herdr:qwen"
seq = time.time_ns()

def send(method, extra, report_seq):
    params = {
        "pane_id": pane_id,
        "source": source,
        "agent": "qwen",
        "seq": report_seq,
    }
    params.update(extra)
    request = json.dumps({
        "id": f"{source}:{report_seq}",
        "method": method,
        "params": params,
    })
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.settimeout(0.5)
            client.connect(os.environ["HERDR_SOCKET_PATH"])
            client.sendall((request + "\n").encode())
            try:
                client.recv(4096)
            except Exception:
                pass
    except Exception:
        pass

if action == "session":
    if session_id is None:
        raise SystemExit(0)
    session_params = {"agent_session_id": session_id}
    if session_start_source is not None:
        session_params["session_start_source"] = session_start_source
    send("pane.report_agent_session", session_params, seq)
    send(
        "pane.report_agent",
        {"state": "idle", "agent_session_id": session_id},
        seq + 1,
    )
elif action == "release":
    send("pane.release_agent", {}, seq)
else:
    state_params = {"state": action}
    if session_id is not None:
        state_params["agent_session_id"] = session_id
    send("pane.report_agent", state_params, seq)
' "$action" 2>/dev/null || true
