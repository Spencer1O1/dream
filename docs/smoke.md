# Catalog smoke — `examples/limits`

Scratch. 2026-08-20. `--strict --no-warn --fresh --build --run`.

```bash
docs/smoke-run.sh
# dream examples/limits/limits.foo -t NAME -o /tmp/dream-smoke/NAME --strict --no-warn --fresh --build --run
```

Expected program stdout when the row can exec:

```
far
origin
near
```

`label_of(3,4)` → 9+16=25 → far; `(0,0)` → origin; `(-1,2)` → 1+4=5 → near.

**Compose is the bar.** Missing compilers are expected. A row passes compose when the store toolchain is the catalog name and each of `limits.foo` / `label.foo` / `score.foo` owns its own source file (no mix).

## Compose

| Row | Store | Units | Files |
|---|---|---|---|
| `cargo` | `cargo` | 3/3 | `src/main.rs`, `src/label.rs`, `src/score.rs` + `Cargo.toml` |
| `go` | `go` | 3/3 | `limits.go`, `label.go`, `score.go` + `go.mod` |
| `python` | `python` | 3/3 | `limits.py`, `label.py`, `score.py` + `pyproject.toml` |
| `node` | `node` | 3/3 | `limits.js`, `label.js`, `score.js` + `package.json` |
| `bun` | `bun` | 3/3 | `limits.js`, `label.js`, `score.js` + `package.json` |
| `deno` | `deno` | 3/3 | `limits.ts`, `label.ts`, `score.ts` + `deno.json` |
| `ruby` | `ruby` | 3/3 | `limits.rb`, `label.rb`, `score.rb` + `Gemfile` |
| `php` | `php` | 3/3 | `limits.php`, `label.php`, `score.php` + `composer.json` |
| `dart` | `dart` | 3/3 | `bin/limits.dart`, `lib/label.dart`, `lib/score.dart` + `pubspec.yaml` |
| `zig` | `zig` | 3/3 | `limits.zig`, `label.zig`, `score.zig` + `build.zig` / `build.zig.zon` |
| `cmake` | `cmake` | 3/3 | `limits.c`, `label.c`, `score.c` + `CMakeLists.txt` |
| `maven` | `maven` | 3/3 | `App.java`, `Label.java`, `Score.java` + `pom.xml` |
| `gradle` | `gradle` | 3/3 | `App.java`, `Label.java`, `Score.java` + `build.gradle.kts` |
| `dotnet` | `dotnet` | 3/3 | `limits.cs`, `label.cs`, `score.cs` + `App.csproj` |
| `swift` | `swift` | 3/3 | `Sources/App/main.swift`, `Label.swift`, `Score.swift` + `Package.swift` |
| `elixir` | `elixir` | 3/3 | `lib/app.ex`, `lib/label.ex`, `lib/score.ex` + `mix.exs` |
| `haskell` | `haskell` | 3/3 | `Main.hs`, `Label.hs`, `Score.hs` + `app.cabal` |
| `nim` | `nim` | 3/3 | `limits.nim`, `label.nim`, `score.nim` + `app.nimble` |
| `crystal` | `crystal` | 3/3 | `limits.cr`, `label.cr`, `score.cr` + `shard.yml` |
| `lua` | `lua` | 3/3 | `limits.lua`, `label.lua`, `score.lua` |
| `r` | `r` | 3/3 | `limits.R`, `label.R`, `score.R` + `DESCRIPTION` |
| `perl` | `perl` | 3/3 | `limits.pl`, `label.pl`, `score.pl` + `cpanfile` |
| `scala` | `scala` | 3/3 | `App.scala`, `Label.scala`, `Score.scala` + `build.sbt` |
| `ocaml` | `ocaml` | 3/3 | `app.ml`, `label.ml`, `score.ml` + `dune` / `dune-project` |
| `make` | `make` | 3/3 | `limits.c`, `label.c`, `score.c` + `Makefile` |
| `unsupported` | none | 0/3 | empty. Model `dream_error`: “Toolchain is unsupported; cannot generate source files.” |

**25/25 catalog rows composed.** Every one kept three units (last smoke collapsed 24 of 25). Store `toolchain` is the catalog name on all of them.

`unsupported` under `--strict` aborted instead of writing generic unit files. That is a model choice, not a host refusal.

## Exec (only where this host can)

| Row | Host | Ran | Notes |
|---|---|---|---|
| `cargo` | present | **PASS** | `far` / `origin` / `near` |
| `go` | present | **PASS** | same |
| `python` | present | **PASS** | same |
| `perl` | present | **PASS** | same. Last smoke failed on missing `cpanm`; this tree ran. |
| `node` | present | **PASS** | same prints. npm wrote “audited 1 package” on stdout before them. |
| `make` | present | **PASS** | same prints. `make` recipe lines also hit stdout. |
| all other catalog rows | missing | not run | compose still wrote the tree. Helper-not-language (`bundle`, `composer`, `nimble`, `shards`) is the same install-hint lie as last smoke. |

## Totals

- Compose OK: 25 / 25 catalog rows
- Compose fail: `unsupported` only (`--strict` abort)
- Exec PASS (host present): `cargo`, `go`, `python`, `node`, `perl`, `make`
- Collapsed units: 0

Dests: `/tmp/dream-smoke/<row>`. Logs: `/tmp/dream-smoke/logs/`.
