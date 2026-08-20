#!/usr/bin/env python3
"""Turn-end verification. Only bother the agent when something still fails."""

from __future__ import annotations

import json
import sys

from verify_lib import (
    clear_dirty,
    clear_failures,
    combined_failure_message,
    dirty,
    load_config,
    set_failure,
    verify_bucket,
)


def main() -> int:
    payload = json.load(sys.stdin)
    if payload.get("status") != "completed":
        print("{}")
        return 0

    config = load_config()
    buckets = config.get("buckets") or {}

    for bucket in buckets:
        flag = f"{bucket}_dirty"
        if not dirty(flag):
            continue
        message = verify_bucket(config, bucket)
        set_failure(bucket, message)
        if not message:
            clear_dirty(flag)

    failure = combined_failure_message()
    if failure:
        followup = (
            "Verification failed before this task can be considered done:\n\n"
            f"{failure}\n\n"
            "Fix the issues above and continue."
        )
        print(json.dumps({"followup_message": followup}))
    else:
        clear_failures()
        print("{}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
