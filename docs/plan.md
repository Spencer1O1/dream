# Plan

Progress lives here. The vault (`MVP.md`, `Core Rules.md`) is the implemented v0 contract. The next contract is vault `Artifact Ownership.md`.

A Dream project is the directory around the entry `.foo`. The model lists and reads other units. There is no manifest and no all-files CLI.

## Phase 1 — Interpreter

```bash
dream now [--strict] <file.foo>
```

- [x] CLI (`now`, `--strict`, compose flags parse)
- [x] Config (`.env` / `.env.local`, `OPENAI_API_KEY`, `DREAM_MODEL`, `DREAM_TURN_CAP`)
- [x] OpenAI Responses tool loop
- [x] `list_source_files` / `read_source_file`
- [x] `stdout` / `stdin`
- [x] Chat text discarded
- [x] `--strict` as a prompt flag
- [x] `DreamError` subtypes (Interpreter / Runtime / Config / Usage)
- [x] Live: `examples/hello.foo`, `examples/hey-you.foo`

## Phase 2 — Multi-file

- [x] Project root = parent of the entry file
- [x] Sandboxed list/read
- [x] Request-loop cycle detection
- [x] Recorded dependency set for the run
- [x] Live: `examples/multifile.foo` + `examples/utils.foo`

## Phase 3 — Compose

```bash
dream [--strict] <file.foo> -t <target> -o <dir>
```

- [x] Composer tool loop (source + composer + control; no stdout/stdin)
- [x] `write_output_file`
- [x] `remove_output_file`
- [x] Stage, then replace `-o` (failed compose leaves the destination)
- [x] Open-ended `-t`
- [x] Live: `hey-you.foo -t rust -o ./out` then `cargo run`

## Phase 4 — Known builders

Declare the toolchain before Dream can build. `-t` stays an open-ended compose hint. A **builder** is a toolchain Dream will exec.

**v0:** asked after the write loop. Phase 7 asks **before** writes.

No pick, or `unsupported`, means do not `--build`, `--run`, or repair. Compose still succeeds.

Do not infer the builder from the tree. Do not take build argv from the model.

The source of truth is `src/builder/catalog.rs`: name, build argv, run argv, install hint. `set_builder` is those names plus `unsupported`. `unsupported` is not a catalog row.

- [x] Closed builder list in Dream (`src/builder/catalog.rs`)
- [x] Follow-up `set_builder` turn after compose settles
- [x] Missing / `unsupported` → compose only

## Phase 5 — Build and run

```bash
dream <file.foo> -t <target> -o <dir> --build
dream <file.foo> -t <target> -o <dir> --run
```

Needs Phase 4. v0 still replaces `-o` first. Then Dream execs the catalog argv in that folder. Build captures streams (repair / `--no-warn`). Run inherits the terminal.

If the declared builder’s toolchain is not on the machine, Dream errors and says how to install it. Dream does not install it. That is different from `unsupported` (Dream has no builder).

- [x] `--build` after a settled compose, only if a known builder was declared
- [x] `--run` implies build
- [x] Capture build IO; inherit run IO
- [x] Missing toolchain → error + install hint (do not auto-install)

## Phase 6 — Bounded repair

Needs Phase 5. Build failures only (not run, not missing toolchain, not `unsupported`). Compose has already replaced `-o`. Repair writes stay in `-o`. Same builder; do not ask again. Warnings are not repair unless `--no-warn`.

Cap is `DREAM_REPAIR_CAP` (default 3, `0` means no repair).

- [x] Build diagnostics back into the composer
- [x] Explicit repair attempt cap
- [x] Do not repair run / missing toolchain / `unsupported`
- [x] `--no-warn` treats toolchain warnings as a failed build

## Phase 7 — Builder first

Needs Phase 6. Still v0 replace-`-o` is fine for this phase.

Ask `set_builder` **once, before any output writes**. That turn has only `set_builder` (and `dream_error`). Then the write loop runs with a known builder (or `unsupported`).

Do not infer the builder from `-t` or from the tree. Do not put `set_builder` in the write-loop catalog.

- [x] `set_builder` before `write_output_file`
- [x] `unsupported` / no pick → compose only, as today

## Phase 8 — Provenance and in-place reconcile

Needs Phase 7. This is the contract break. See vault `Artifact Ownership.md`.

Normal `dream` must not clear `-o`. Persist a minimal target-specific map in a Dream-owned file under `-o`. Writes belong to the compose-stack current unit. `read_source_file` of an unlocked unsettled unit recurses and composes that unit first. Locked or already-settled reads return foocode + artifacts.

Reject overwrite of another unit, project-owned, or unmanaged path. After a unit settles, delete only that unit’s previous paths that are gone from the new set.

`-o` has files but no store → error (use `--fresh` or an empty dir). Store target ≠ `-t` → error unless `--fresh`.

Repair after Phase 8: stack is empty; only overwrite existing unlocked unit-owned paths; no new files, no `set_dependencies`.

`--fresh` the same day: drop provenance, locks, and Dream-owned paths; leave unmanaged files; recompose.

Do not invent lock CLI, project tools, or a formal IR in this phase. Do not require one `.foo` → one target file. No source-hash skip yet.

- [ ] Stop replace-`-o` on normal compose
- [ ] Persist unit → artifact paths in `-o` (format not precious)
- [ ] Compose stack: writes belong to the current unit
- [ ] Recurse on `read_source_file` of unlocked unsettled units
- [ ] Enforce write / delete against provenance
- [ ] Preserve unmanaged paths
- [ ] No-store-with-files and target-mismatch errors
- [ ] `--fresh` drops Dream-owned only

## Phase 9 — Project layer

Needs Phase 8. Known builders only.

Dream owns manifests. One tool: `set_dependencies` for the current unit — package names plus optional features; Dream chooses versions. Package name = entry stem on init only; do not overwrite later. Dream does not generate target-language wiring (`mod` / `import` / …). `unsupported`: first writer owns manifest-shaped files.

- [ ] Dream-owned manifest mutation for catalog builders
- [ ] Composer writes of those paths rejected
- [ ] `set_dependencies` replaces the current unit’s claims (name + optional features)
- [ ] Init package name from entry stem; leave existing names alone

## Phase 10 — Target-specific locks

Needs Phase 8. Phase 9 preferred so locked units do not imply frozen stale manifests.

```bash
dream lock <file.foo> -t <target>
```

Freezes that unit’s current artifact set and contents for that target. Normal `dream` skips those artifacts. No `redream` command. Unlock, then `dream`, to recompose. `--fresh` ignores locks.

Inspect is later if needed.

- [ ] Lock / unlock a unit for a target (store source hash)
- [ ] Normal reconcile skips locked units
- [ ] Compose `read_source_file` of a locked unit returns foocode plus frozen artifacts
- [ ] `dream now` reads stay `{ path, source }` only
- [ ] Locked source hash mismatch or missing artifact → error
- [ ] Hand-edited locked artifacts stay; Composer still will not write them

## Later

Do not start these during the phases above.

- [ ] Skip unchanged unlocked units (source hash)
- [ ] `dream inspect`
- [ ] `dream.toml` (name / entry so `dream .` works)
- [ ] Formal semantic core / Gimbal
- [ ] Target-independent locks
- [ ] Deterministic `dream now` runtime
- [ ] Data-file / HTTP tools
- [ ] Provider plugins
