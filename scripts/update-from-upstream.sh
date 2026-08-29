#!/usr/bin/env bash
# Compatibility wrapper for the cross-platform, merge-based sync tool.

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
exec python3 "$PROJECT_DIR/scripts/sync_upstream.py" "$@"
