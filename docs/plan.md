# Plan

Progress lives here. The vault (`MVP.md`, `Core Rules.md`) is the product contract, not a checklist.

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

After the write loop settles, Dream asks once for a builder. That turn has only `set_builder` (and `dream_error`). No pick, or `unsupported`, means do not `--build`, `--run`, or repair. Compose still succeeds.

Do not infer the builder from the tree. Do not take build argv from the model. Do not put `set_builder` in the write-loop catalog.

The source of truth is `src/builder/catalog.rs`: name, build argv, run argv, install hint. `set_builder` is those names plus `unsupported`. `unsupported` is not a catalog row.

Enum values are toolchains Dream owns, not language vibes (`cpp`, `embedded`). Vague `-t` (Arduino Nano, COBOL, …) should be `unsupported` until that builder exists.

- [x] Closed builder list in Dream (`src/builder/catalog.rs`)
- [x] Follow-up `set_builder` turn after compose settles
- [x] Missing / `unsupported` → compose only

## Phase 5 — Build and run

```bash
dream <file.foo> -t <target> -o <dir> --build
dream <file.foo> -t <target> -o <dir> --run
```

Needs Phase 4. Compose still replaces `-o` first. Then Dream execs the catalog argv in that folder and inherits stdin/stdout/stderr.

If the declared builder’s toolchain is not on the machine, Dream errors and says how to install it. Dream does not install it. That is different from `unsupported` (Dream has no builder).

- [x] `--build` after a settled compose, only if a known builder was declared
- [x] `--run` implies build
- [x] Forward standard process IO
- [x] Missing toolchain → error + install hint (do not auto-install)

## Phase 6 — Bounded repair

Needs Phase 5. Build failures only (not run, not missing toolchain, not `unsupported`). Compose has already replaced `-o`. Repair writes stay in `-o`. Same builder; do not ask again. Warnings are not repair unless `--no-warn`.

```text
build
↓
diagnostics
↓
composer repair (same write tools)
↓
build
```

Cap is `DREAM_REPAIR_CAP` (default 3, `0` means no repair).

- [x] Build diagnostics back into the composer
- [x] Explicit repair attempt cap
- [x] Do not repair run / missing toolchain / `unsupported`
- [x] `--no-warn` treats toolchain warnings as a failed build

## Phase 7 — Semantic cache foundations

File-level only. No formal IR.

- [ ] Canonical path, source hash, semantic status

## Later

Do not start these during the MVP phases.

- [ ] `dream.toml` (name / entry so `dream .` works)
- [ ] Semantic lock / inspect
- [ ] Incremental re-dream
- [ ] Formal semantic core
- [ ] Deterministic `dream now` runtime
- [ ] Data-file / HTTP tools
- [ ] Provider plugins
