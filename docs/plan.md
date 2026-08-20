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

## Phase 4 — Build and run

```bash
dream <file.foo> -t <target> -o <dir> --build
dream <file.foo> -t <target> -o <dir> --run
```

- [ ] `--build` after a settled compose
- [ ] `--run` implies build
- [ ] Forward standard process IO

CLI already accepts the flags and errors: composition does not build yet.

## Phase 5 — Known builders

- [ ] Toolchains for common targets (Rust, Go, …)
- [ ] Unknown `-t` still composes; build may fail if Dream has no builder

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
