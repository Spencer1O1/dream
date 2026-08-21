# Audit checklist

Scratch. From the three auditors after composer-owned setup.

## Dest and catalog

- [x] Wipe dirs include children (`target/foo.rs` is wipe, not a unit file)
- [x] `store.owner` treats wipe-dir children as project
- [x] `--fresh` / `drop_owned` deletes wipe dirs (use dest remove, not file-only)
- [x] `perl` configure fetches from `cpanfile`
- [x] No `installed` / `Dependency` / `set_dependencies` leftovers

## Store toolchain and pick

- [x] Store toolchain is the catalog name (`cargo`), not the `-t` target (`rust`)
- [x] Exact `-t` must match the store row (`-t go` vs store `cargo` fails)
- [x] Fuzzy `-t` on an existing catalog store reuses that row (no pick)
- [x] Silent pick (no `set_toolchain`) is an error, not compose with no toolchain
- [x] Second `set_toolchain` on the pick turn is refused
- [x] Pick-turn tool calls are not left in the write-loop conversation
- [x] Write loop gets a labeled toolchain user blob (`setup` / `project` / `entrypoint` / Dream execs)

## Compose and repair pipelines

- [x] `require_composed` only fails a first materialize with no unit files
- [x] Incremental no-write settle is allowed
- [x] Repair is a new empty stack (diagnostics + toolchain fact)
- [x] Repair still only on configure/build, not run or missing program
- [x] Dead `build failed` tail after the repair loop is gone

## Agent surface

- [x] Compose dest rules: setup writes pass the entry `.foo`; wipe `project` is do not write
- [x] No-setup rows (`lua` / `unsupported`) are not told to write setup
- [x] Repair dest rules do not say dest files name a `.foo`
- [x] Repair goal does not say “existing output files”
- [x] Lock line does not say “dependencies”
- [x] `unit` parameter text: setup uses the entry `.foo`
- [x] Dest path parameters say dest-relative (not “in the project”)
- [x] `read_file` (composer) description is just dest read
- [x] Refusals say dest / wipe / user-owned — not “output” or “Dream owns that path” for wipe
- [x] Missed `.foo`, dest escape, lucid `read_file` on a `.foo` are warnings, not session abort
- [x] Pick preamble: pick the catalog row or `unsupported`
- [x] Clap `-o` / `--fresh` say dest, not output
- [x] Shared foocode line says `.foo` file

## Verify

- [x] `cargo fmt`
- [x] `cargo clippy -- -D warnings`
- [x] `cargo test`
