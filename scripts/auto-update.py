#!/usr/bin/env python3
"""Deprecated compatibility entry point for safe upstream synchronization."""

from __future__ import annotations

import sys

from sync_upstream import main


if __name__ == "__main__":
    print(
        "scripts/auto-update.py now performs sync only; "
        "use scripts/herdr_deploy.py or the fork workflow for deployment.",
        file=sys.stderr,
    )
    raise SystemExit(main())
