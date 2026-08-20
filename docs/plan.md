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

The composer declares one with `set_builder` (anytime, last call wins). Never called, or `unsupported`, means do not `--build`, `--run`, or repair. Compose still succeeds.

Do not infer the builder from the tree. Do not take build argv from the model. Do not add a `finish` tool; settle stays “no more tool calls.”

Enum values are toolchains Dream owns (`cargo`, `go`, …), not language vibes (`cpp`, `embedded`). Vague `-t` (Arduino Nano, COBOL, …) should be `unsupported` until that builder exists.

- [ ] Closed builder list in Dream
- [ ] Composer tool `set_builder`
- [ ] Missing / `unsupported` → compose only

## Phase 5 — Build and run

```bash
dream <file.foo> -t <target> -o <dir> --build
dream <file.foo> -t <target> -o <dir> --run
```

Needs Phase 4. Worth doing for **repair** (Phase 7), not as a `cargo` wrapper. Until then, the user builds the folder themselves. CLI flags already parse and error.

- [ ] `--build` after a settled compose, only if a known builder was declared
- [ ] `--run` implies build
- [ ] Forward standard process IO

## Phase 6 — Semantic cache foundations

File-level only. No formal IR.

- [ ] Canonical path, source hash, semantic status

## Phase 7 — Bounded repair

- [ ] Build diagnostics back into the composer
- [ ] Explicit repair attempt cap

## Later

Do not start these during the MVP phases.

- [ ] `dream.toml` (name / entry so `dream .` works)
- [ ] Semantic lock / inspect
- [ ] Incremental re-dream
- [ ] Formal semantic core
- [ ] Deterministic `dream now` runtime
- [ ] Data-file / HTTP tools
- [ ] Provider plugins
