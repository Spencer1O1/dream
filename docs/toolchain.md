# Toolchain

Scratch. Vault stays law.

## Architecture (do not fork this)

One compose path. No interpreted-vs-compiled pipeline.

```text
set_toolchain          → docs, setup, project (wipe), configure/build/run, entrypoint.path
write_file             → unit-owned sources, plus this row’s setup files
Dream execs the row    → configure / build / run
```

The **catalog row** is the abstraction. It names exec argv, setup (composer-writable), wipe paths, docs, and `entry`. Dream does not write manifests. Exec never takes argv from the model.

Three jobs. Do not merge them:

1. **Program start** — `entry` → `entrypoint.path`. Stem or fixed. The agent writes that file.
2. **How Dream execs** — configure / build / run on the row. The model **sees** those argv as a fact.
3. **Which files compile** — the toolchain, or the setup file the agent wrote. Writing `util.c` is the declaration. No source-list tool.

Still refuse: a second pipeline by language class, taking argv from the model, Dream-generated `mod` / `import` / `#include`.

## How each row finds sources

| Row | Entry | Other sources |
|---|---|---|
| python, node, bun, deno, ruby, php, lua, r, perl | `{stem}.*` | Imports from that file. |
| go | `{stem}.go` | Package dir (`go run .`). |
| dart | `bin/{stem}.dart` | `lib/` by convention. |
| dotnet | `{stem}.cs` | SDK globs `**/*.cs`. |
| maven, gradle | `src/main/java/App.java` | Tree under `src/main/java`. |
| scala | `src/main/scala/App.scala` | Tree under `src/main/scala`. |
| swift | `Sources/App/main.swift` | `Sources/App/`. |
| elixir | `lib/app.ex` | `lib/`. |
| cargo | `src/main.rs` | `mod` / `use` in source. |
| zig | `{stem}.zig` | `@import` from the `build.zig` root. |
| nim, crystal | `{stem}.nim` / `{stem}.cr` | Imports / `require` from the entry. |
| haskell | `Main.hs` | Cabal / other modules in dest. |
| ocaml | `app.ml` | Dune infers `.ml` next to `dune`. |
| cmake, make | `{stem}.c` | Whatever the Makefile / CMakeLists lists. Make has no package manager. |

Composer writes setup. `--fresh` drops setup and wipe. Init marks those paths and writes no contents.

## Fetch

- cargo / go: `build` fetches.
- node / bun / ruby / php: `configure` is `npm install` / `bun install` / `bundle install` / `composer install`.
- cmake: existing `cmake -S . -B build`.
- python / perl / make / lua: empty configure. `cpanfile` is setup, not a fetch.

## Loop

`run → fix → repeat` on `examples/limits/` for every catalog name. Missing compiler + install hint is a finding. Wrong dest or a pick miss is a Dream bug.

- Do not mkdir `target/` / `bin/` / `_build` at init. Create them at exec.
- If `-t` is a catalog name, bind it. Do not lottery `set_toolchain`.
- No-setup rows (`lua`) still settle.

## Later (not this file’s job)

Composer MCP. Repair LSP. Not Gimbal, not `dream now`, not provider plugins.
