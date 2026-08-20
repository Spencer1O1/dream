#!/usr/bin/env python3
"""Shared helpers for portable Cursor verify hooks (config-driven)."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any

HOOKS_DIR = Path(__file__).resolve().parent
ROOT = HOOKS_DIR.parents[1]
STATE_DIR = HOOKS_DIR / "state"
FAILURE_STATE = STATE_DIR / "failures.json"
CONFIG_PATH = HOOKS_DIR / "verify.config.json"
MAX_OUTPUT_CHARS = 6000


def load_config() -> dict[str, Any]:
    if not CONFIG_PATH.is_file():
        return {}
    return json.loads(CONFIG_PATH.read_text(encoding="utf-8"))


def rel_path(file_path: str) -> str | None:
    path = Path(file_path)
    try:
        return path.resolve().relative_to(ROOT.resolve()).as_posix()
    except ValueError:
        return None


def touch_dirty(name: str) -> None:
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    (STATE_DIR / name).touch()


def dirty(name: str) -> bool:
    return (STATE_DIR / name).is_file()


def clear_dirty(name: str) -> None:
    path = STATE_DIR / name
    if path.is_file():
        path.unlink()


def clear_failures() -> None:
    if FAILURE_STATE.is_file():
        FAILURE_STATE.unlink()


def load_failures() -> dict[str, str]:
    if not FAILURE_STATE.is_file():
        return {}
    try:
        data = json.loads(FAILURE_STATE.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {}
    if not isinstance(data, dict):
        return {}
    return {
        key: value
        for key, value in data.items()
        if isinstance(value, str) and value
    }


def save_failures(failures: dict[str, str]) -> None:
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    active = {key: value for key, value in failures.items() if value}
    if active:
        FAILURE_STATE.write_text(
            json.dumps(active, indent=2) + "\n", encoding="utf-8"
        )
    elif FAILURE_STATE.is_file():
        FAILURE_STATE.unlink()


def set_failure(bucket: str, message: str | None) -> None:
    failures = load_failures()
    if message:
        failures[bucket] = message
    else:
        failures.pop(bucket, None)
    save_failures(failures)


def combined_failure_message() -> str | None:
    failures = load_failures()
    if not failures:
        return None
    return "\n\n".join(failures.values())


def matches_bucket(rel: str, bucket_cfg: dict[str, Any]) -> bool:
    path = Path(rel)
    for prefix in bucket_cfg.get("exclude_prefixes") or []:
        if rel.startswith(prefix):
            return False
    names = set(bucket_cfg.get("extra_names") or [])
    if rel in names or path.name in names:
        return True
    suffixes = set(bucket_cfg.get("path_suffixes") or [])
    if suffixes and path.suffix not in suffixes:
        return False
    prefixes = bucket_cfg.get("path_prefixes") or []
    if not prefixes:
        return bool(suffixes)
    return any(rel.startswith(p) for p in prefixes)


def augment_path(env: dict[str, str]) -> dict[str, str]:
    home = Path.home()
    prefixes: list[str] = [
        "/usr/bin",
        "/bin",
        "/usr/local/bin",
        "/usr/local/go/bin",
    ]
    # Windows / user local
    local_bin = home / ".local" / "bin"
    if local_bin.is_dir():
        prefixes.append(str(local_bin))
    go_bin = home / "go" / "bin"
    if go_bin.is_dir():
        prefixes.append(str(go_bin))
    cargo_bin = home / ".cargo" / "bin"
    if cargo_bin.is_dir():
        prefixes.append(str(cargo_bin))
    nvm = home / ".nvm" / "versions" / "node"
    if nvm.is_dir():
        versions = sorted(nvm.iterdir(), key=lambda p: p.name)
        if versions:
            prefixes.append(str(versions[-1] / "bin"))
    prefixes.append(str(home / ".local" / "share" / "pnpm"))
    appdata = os.environ.get("APPDATA")
    if appdata:
        prefixes.append(str(Path(appdata) / "npm"))
    env = env.copy()
    env["PATH"] = os.pathsep.join(prefixes + [env.get("PATH", "")])
    return env


def run(command: list[str], cwd: Path = ROOT) -> tuple[int, str]:
    env = augment_path(os.environ)
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as exc:
        return 127, str(exc)
    output = (completed.stdout + completed.stderr).strip()
    return completed.returncode, output


def trim(output: str) -> str:
    if len(output) <= MAX_OUTPUT_CHARS:
        return output
    return output[-MAX_OUTPUT_CHARS:]


def cmd(config: dict[str, Any], key: str) -> list[str] | None:
    commands = config.get("commands") or {}
    value = commands.get(key)
    if not value:
        return None
    if not isinstance(value, list) or not all(isinstance(x, str) for x in value):
        return None
    return value


def check_named(config: dict[str, Any], name: str) -> str | None:
    key = {
        "format": "format_check",
        "lint": "lint",
        "typecheck": "typecheck",
    }.get(name, name)
    command = cmd(config, key)
    if not command:
        return None
    code, output = run(command)
    if code == 0:
        return None
    labels = {
        "format": "Format check failed after auto-format. Fix formatting before finishing.",
        "lint": "Lint failed. Fix every reported error before finishing.",
        "typecheck": "Typecheck failed. Fix every reported error before finishing.",
    }
    label = labels.get(name, f"{name} failed. Fix before finishing.")
    return f"{label}\n\n```\n{trim(output)}\n```"


def verify_bucket(config: dict[str, Any], bucket: str) -> str | None:
    buckets = config.get("buckets") or {}
    bucket_cfg = buckets.get(bucket) or {}
    checks = bucket_cfg.get("checks") or [bucket]
    for check in checks:
        message = check_named(config, check)
        if message:
            return message
    return None


def auto_format_file(config: dict[str, Any], rel: str) -> None:
    auto = config.get("auto_format") or {}
    if not auto.get("enabled", True):
        return
    abs_path = ROOT / rel
    if not abs_path.is_file():
        return
    suffixes = set(auto.get("suffixes") or [])
    if abs_path.suffix not in suffixes:
        return
    write_cmd = cmd(config, "format_write")
    if not write_cmd:
        return
    run([*write_cmd, rel])


def dirty_buckets_for(config: dict[str, Any], rel: str) -> list[str]:
    buckets = config.get("buckets") or {}
    hit: list[str] = []
    for name, bucket_cfg in buckets.items():
        if isinstance(bucket_cfg, dict) and matches_bucket(rel, bucket_cfg):
            hit.append(name)
    return hit
