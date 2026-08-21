# Plan

Progress lives here. Vault `Core Rules.md` is foundational. `--lucid` still matches vault `MVP.md` interpreter behavior. Compose matches vault `Artifact Ownership.md` through Phase 10.

A Dream project is the directory around the entry `.foo`, or a directory with `dream.toml` `[project] entry`. The model lists and reads other units. There is no all-files CLI.

## Phase 1 — Interpreter

```bash
dream [--lucid] [--strict] <file.foo>
```

- [x] CLI (`--lucid`, `--strict`, compose flags parse)
- [x] Config (`.env` / `.env.local`, `OPENAI_API_KEY`, `DREAM_MODEL`, `DREAM_TURN_CAP`)
- [x] OpenAI Responses tool loop
- [x] `list_source_files` / `read_source_file`
- [x] `stdout` / `stdin`
- [x] Chat text discarded
- [x] `--strict` as a prompt flag
- [x] `DreamError` subtypes (`src/error`)
- [x] Live: `examples/hello/hello.foo`, `examples/hey-you/hey-you.foo`

## Phase 2 — Multi-file

- [x] Project root = parent of the entry file
- [x] Sandboxed list/read
- [x] Recorded read set for the run (entry + units that were read; re-read is fine)
- [x] No request-stack cycle abort (that blocked compose reads of the entry)
- [x] Live: `examples/multifile/multifile.foo` + `examples/multifile/utils.foo`

## Phase 3–7 — Historical

These phases shipped **replace-`-o`** and, until Phase 7, **toolchain last**. Phase 8 is the contract break: compose writes in place. Do not restore replace-`-o` or a post-write `set_toolchain` turn.

## Phase 3 — Compose

```bash
dream [--strict] <file.foo> -t <target> -o <dir>
```

- [x] Composer tool loop (source + composer + control; no stdout/stdin)
- [x] `write_file`
- [x] `remove_file`
- [x] Historical: stage, then replace `-o` (failed compose leaves the destination). Replaced in Phase 8.
- [x] Open-ended `-t`
- [x] Live: `hey-you/hey-you.foo -t rust -o ./out` then `cargo run`

## Phase 4 — Known toolchains

Declare the toolchain before Dream can `--build` or `--run`. `-t` stays an open-ended compose hint. A **toolchain** is a catalog row Dream will exec (`cargo`, `go`, `python`), not a language vibe.

Historical: asked after the write loop. Phase 7 asks **before** writes.

No pick, or `unsupported`, means do not `--build`, `--run`, or repair. Compose still succeeds.

Do not infer the toolchain from the tree. Do not take build or run argv from the model.

The source of truth is `src/toolchain/catalog.rs`: name, optional configure/build argv, run argv, install hint, official docs URL, optional manifest writer, project paths. Add a language by writing `create`/`apply` and one catalog row. `set_toolchain` is those names plus `unsupported`. `unsupported` is not a catalog row. `set_toolchain` replies with `docs` so the model can open the official docs. That is not a fetch.

- [x] Closed toolchain catalog in Dream (`src/toolchain/catalog.rs`)
- [x] Historical: follow-up `set_toolchain` turn after compose settles. Phase 7 asks first.
- [x] Missing / `unsupported` → compose only

## Phase 5 — Build and run

```bash
dream <file.foo> -t <target> -o <dir> --build
dream <file.foo> -t <target> -o <dir> --run
```

Needs Phase 4. Historical: replaced `-o` first, then exec. Phase 8 builds in the in-place dest. Build captures streams (repair / `--no-warn`). Run inherits the terminal.

If the declared toolchain is not on the machine, Dream errors and says how to install it. Dream does not install it. That is different from `unsupported` (Dream has no catalog row).

- [x] `--build` after a settled compose, only if a known toolchain was declared
- [x] `--run` implies build
- [x] Capture build IO; inherit run IO
- [x] Missing toolchain → error + install hint (do not auto-install)

## Phase 6 — Bounded repair

Needs Phase 5. Build failures only (not run, not missing toolchain, not `unsupported`). Historical: compose had already replaced `-o`. Repair writes stay in `-o`. Same toolchain; do not ask again. Warnings are not repair unless `--no-warn`.

Cap is `DREAM_REPAIR_CAP` (default 3, `0` means no repair).

- [x] Build diagnostics back into the composer
- [x] Explicit repair attempt cap
- [x] Do not repair run / missing toolchain / `unsupported`
- [x] `--no-warn` treats toolchain warnings as a failed build

## Phase 7 — Toolchain first

Needs Phase 6. Historical: still used replace-`-o`. Phase 8 is the contract break.

Ask `set_toolchain` **once, before any output writes**. That turn has only `set_toolchain` (no `dream_error`, no write tools). Then the write loop runs with a known toolchain (or `unsupported`).

Do not infer the toolchain from `-t` or from the tree. Do not put `set_toolchain` in the write-loop catalog.

- [x] `set_toolchain` before `write_file`
- [x] `unsupported` / no pick → compose only, as today

## Phase 8 — Provenance and in-place reconcile

Needs Phase 7. This is the contract break. See vault `Artifact Ownership.md`.

Normal `dream` must not clear `-o`. Persist a minimal target-specific map in a Dream-owned file under `-o`. One composition session from the entry. `read_source_file` never composes; it returns foocode plus stored artifacts if any. Writes name the owning `.foo`. Dream checks the claim (unit exists, was read or is the entry, not stolen / project / user-owned).

Reject overwrite of another unit, project-owned, or unmanaged path. After the session settles, delete only each writing unit’s previous paths that are gone from its new set.

`-o` has files but no store → error (use `--fresh` or an empty dir). Any file outside `.dream/` counts, including leftover `target/`. Do not skip toolchain dirs by name. Store target ≠ `-t` → error unless `--fresh`.

Repair after Phase 8: stack is empty; only overwrite existing unlocked unit-owned paths; no new files, no `set_dependencies`.

`--fresh` the same day: drop provenance, locks, and Dream-owned paths; leave unmanaged files; recompose.

Do not invent lock CLI, project tools, or a formal IR in this phase. Do not require one `.foo` → one target file. Do not skip unlocked units because the source hash matches. Lock is the skip.

- [x] Stop replace-`-o` on normal compose
- [x] Persist unit → artifact paths in `-o` (format not precious)
- [x] Compose stack: writes belong to the current unit (replaced: write names `unit`)
- [x] Recurse on `read_source_file` of unlocked unsettled units (replaced: read never composes)
- [x] Write / remove require `unit`; Dream checks the claim
- [x] One composition session; no nested job
- [x] Enforce write / delete against provenance
- [x] Preserve unmanaged paths
- [x] No-store-with-files and target-mismatch errors
- [x] `--fresh` drops Dream-owned only

## Phase 9 — Project layer

Needs Phase 8. Known toolchains only.

Dream owns manifests. One tool: `set_dependencies(unit, …)` — package names plus optional version and features. Package name = entry stem on init only; do not overwrite later. Dream does not generate target-language wiring (`mod` / `import` / …). `unsupported`: first writer owns manifest-shaped files.

- [x] Dream-owned manifest mutation for catalog toolchains
- [x] Composer writes of those paths rejected
- [x] `set_dependencies` replaces that unit’s dependencies (name + optional version and features)
- [x] Init package name from entry stem; leave existing names alone

## Phase 10 — Target-specific locks

Needs Phase 8. Phase 9 preferred so locked units do not imply frozen stale manifests.

```bash
dream lock <file.foo> -t <target> -o <dir>
dream unlock <file.foo> -t <target> -o <dir>
```

Freezes that unit’s current artifact **set** and the `.foo` source hash for that dest. Contents stay on disk; Dream does not snapshot file bodies. Writes, removes, and `set_dependencies` for a locked unit are rejected. Unreached locked units stay. No `redream` command. Unlock, then `dream`, to recompose. `--fresh` ignores locks.

- [x] Lock / unlock a unit for a target (store source hash)
- [x] Normal reconcile skips locked units
- [x] Compose `read_source_file` of a locked unit returns foocode plus frozen artifacts
- [x] `--lucid` reads stay `{ path, source }` only
- [x] Locked source hash mismatch or missing artifact → error
- [x] Hand-edited locked artifacts stay; Composer still will not write them

## After Phase 10

- [x] Catalog rows (exec + manifest writer + project wipe + docs URL). `lua` / `make` have no manifest.
- [x] `dream inspect <file.foo|.> -t <target> -o <dir>` — human stdout, no LLM
- [x] `dream.toml` `[project] name` / `entry` so `dream .` resolves the entry
- [x] Lucid `list_files` / `read_file` / `write_file` (project sandbox; not `.foo`; not `.dream`)
- [x] Lucid `http_request` (the program on the network; Dream performs)

## Later

- [ ] Composer research via MCP (indexes, docs). Dream owns the fetch. Not a version resolver. Catalog `docs` URL is already on `set_toolchain`.
- [ ] Composer and repair targeted / LSP edits. `write_file` stays whole-file replace until then.

Not queued: Gimbal / target-independent locks, deterministic `--lucid` (`dream now`). Provider plugins.
