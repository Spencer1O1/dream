#!/usr/bin/env python3
"""Auto-format edited files; defer heavy checks to the stop hook."""

from __future__ import annotations

import json
import sys

from verify_lib import (
    auto_format_file,
    clear_failures,
    dirty_buckets_for,
    load_config,
    rel_path,
    touch_dirty,
)


def main() -> int:
    payload = json.load(sys.stdin)
    rel = rel_path(payload.get("file_path", ""))
    if rel is None:
        return 0

    clear_failures()
    config = load_config()
    auto_format_file(config, rel)
    for bucket in dirty_buckets_for(config, rel):
        touch_dirty(f"{bucket}_dirty")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
