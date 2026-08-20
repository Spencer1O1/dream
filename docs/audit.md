# Audit

Auditor pass. Verdict was **fix-first**. Check off as we go.

Effect ownership, preamble-without-tool-names, `--lucid` isolation, and locks-as-flags were judged sound. Do not grow a new subsystem while working these.

## Must-fix

- [x] **DepGraph cycle** — Stop treating last-read as the composer. A re-read of the entry must not abort. Record reads as a set; `reached` is entry-or-read. (`src/source/graph.rs`)

## Architecture

- [x] **tools ↔ composer cycle** — Provenance and dest I/O live at the crate root. `ToolCtx` is project + deps + a `Mode` (Lucid / Pick / Compose / Repair), not an option bag. Tools do not import `composer`.
- [x] **One authorize** — Write / remove / `set_dependencies` go through `authorize` (reached + lock). `--fresh` drops the store at open; it is not a write-time lock bypass.
- [x] **write / remove duplication** — One `mutate_output` helper. Repair registry omits `unit` (owner is the map) and `set_dependencies`.
- [x] **Pick Session** — `ask_builder` is a function. It owns `Registry::builder()`. Compose `Session` is only for compose / repair.
- [x] **reserved / staging** — One `reserved()` in `provenance/store.rs`. Dest I/O params are `dest`.
- [x] **Source list skip** — `list_source_files` returns every project-relative `.foo`. No skip of `target/` or `.`*.



## Prompting

- [x] **Parallel catalog sentence** — Keep the batch instruction. Dropped “Anything else is invalid.” Sequential still works. The listed tools are the interface.
- [x] **Lucid tool text** — Lucid list/read are path/source only. Compose list/read describe `locked` and stored artifacts. Mutation tools still say they fail if the unit is locked.
- [x] **Flags on the wrong turn** — `--no-warn` is build-step only; it is not in prompts. `--strict` is on interpret / compose / repair (`dream_error`). The pick turn has neither.
- [x] **Write/remove description** — Compose write/remove say source (code) under the output root. Fails if that unit is locked. Not “unit owns,” “dest,” or “Dream-owned.” No Cargo.toml filename. Manifest writes are still refused.
- [x] **Warning tool name** — Project-owned write/remove say Dream owns the manifest. They do not name `set_dependencies` (absent on repair / unsupported) or invent “the project dependency tool.” The catalog already names that tool when it exists.



## Taxonomy

- [x] **Lock-staleness error type** — Drifted lock is `ComposerError` on `dream lock` and compose `check`. Same class as a missing locked artifact. Usage stays for “this command does not apply.”
- [x] **Missing remove** — Missing / directory remove is a tool warning. Escape and I/O stay `DreamError`.
- [x] **dream_error name** — `dream_error` and the lucid turn cap are `InterpreterError`. The compose turn cap is `ComposerError`. See `docs/errors.md`.



## Leftovers

- [x] **plan.md Phase 3–7** — replace-`-o` and builder-last are marked historical. Phase 8 is the contract break.
- [x] **tempfile** — `[dev-dependencies]` only.
- [x] **Go features twice** — Tool warning only. `apply` `debug_assert`s; not a second process error.