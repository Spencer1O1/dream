# Audit

Second auditor pass (`-Wpedanticpedanticpedantic`). Verdict: **fix-first**. Previous punch list is closed; this is a new list.

Effect ownership, `Mode` isolation, in-place provenance, `--lucid` isolation, and locks-as-flags were judged sound. Do not grow a new subsystem while working these. Do not add source-hash skip, `inspect`, `dream.toml`, Gimbal, a version resolver, lock CLI extra args, or repair-remove persistence.

## Must-fix

- [x] **Lock nested unit** — `dream lock users/active.foo` re-roots at that file’s parent, so the store key is `active.foo` while compose recorded `users/active.foo`. Match the file to an existing store key. Test a subdirectory unit. (`src/composer/mod.rs`, `src/source/project.rs`, `src/provenance/lock.rs`)
- [x] **Repair remove** — Repair catalog offers `remove_output_file`. `authorize_remove` with `unit: None` always fails; repair does not settle. Vault is overwrite-only. Drop remove from `Registry::repair()`. Do not teach remove to update the map.

## Architecture

- [x] **Go stub versions** — Optional `version` on `set_dependencies`. No version → omit `require`. Do not write `v0.0.0`. No crates.io/proxy resolver.
- [x] **Python `--run`** — Catalog `run` is `["python", "main.py"]`. Static argv; not a REPL; not from the model.

## Prompting

- [x] **Repair preamble** — Repair calls `prompt::compose` (“write a complete project”). New paths are then refused. One repair-only preamble: overwrite existing output files to fix the build. No tool names.

## Taxonomy

- [ ] **Missing locked `.foo`** — Hash mismatch is `ComposerError`. A deleted locked source is `RuntimeError` from `read_source_file`. Map the missing locked unit in `hash_unit` / `check` / `lock`. Do not change tool reads.
- [ ] **Authorize wording** — Shared reached-check says `cannot write for`; `set_dependencies` uses it. Say “read that unit first.” Do not split authorize.

## Leftovers

- [ ] **README contract** — Still says implemented contract is `MVP.md` and Artifact Ownership is next. The crate is Artifact Ownership through Phase 10. Update that sentence.
- [ ] **`interpreter::prompt::compose`** — Lucid instructions are built by a function named `compose`. Rename to `lucid` / `interpret`.
- [ ] **plan.md audit pointer** — Line 3 still points at the closed first punch list as current work.
