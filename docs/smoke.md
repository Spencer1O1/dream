# Catalog smoke — `examples/limits`

Scratch. 2026-08-20 after `aea2a76`.

```bash
dream examples/limits/limits.foo -t NAME -o /tmp/dream-smoke/NAME --fresh --build --run
```

Expected program stdout:

```
far
origin
near
```

`Host` is whether this machine has the row’s first program (or a listed fallback). Missing compiler plus the catalog install hint is expected, not a Dream bug. The first classifier treated `ComposerError: Install …` as `EXEC_FAIL`; those rows are `MISSING_HINT` below.

| Row | Host | Result | Notes |
|---|---|---|---|
| `cargo` | present | PASS | |
| `go` | present | PASS | |
| `python` | present | PASS | |
| `node` | present | PASS | |
| `make` | present | PASS | |
| `perl` | present | **FAIL** | `perl` is here; configure is `cpanm --installdeps .`. `cpanm` is missing. Hint says install Perl. The composed `limits.pl` prints `far` / `origin` / `near` if you run `perl` directly. |
| `bun` | missing | MISSING_HINT | Install Bun |
| `deno` | missing | MISSING_HINT | Install Deno |
| `ruby` | missing | MISSING_HINT | Install Ruby |
| `php` | missing | MISSING_HINT | Install PHP |
| `dart` | missing | MISSING_HINT | Install Dart. Only row that wrote all three units (`bin/limits.dart`, `lib/label.dart`, `lib/score.dart`). |
| `zig` | missing | MISSING_HINT | Install Zig. Wrote `build.zig` + `build.zig.zon` + `limits.zig`. |
| `cmake` | missing | MISSING_HINT | Install CMake. Wrote a `run` custom target (matches catalog). |
| `maven` | missing | MISSING_HINT | Install Maven (`javac` is present; `mvn` is not). |
| `gradle` | missing | MISSING_HINT | Install Gradle |
| `dotnet` | missing | MISSING_HINT | Install .NET |
| `swift` | missing | MISSING_HINT | Install Swift |
| `elixir` | missing | MISSING_HINT | Install Elixir |
| `haskell` | missing | MISSING_HINT | Install Cabal |
| `nim` | missing | MISSING_HINT | Install Nim |
| `crystal` | missing | MISSING_HINT | Install Crystal |
| `lua` | missing | MISSING_HINT | Install Lua |
| `r` | missing | MISSING_HINT | Install R |
| `scala` | missing | MISSING_HINT | Install sbt |
| `ocaml` | missing | MISSING_HINT | Install Dune |

## Totals

- PASS: 5 (`cargo`, `go`, `python`, `node`, `make`)
- FAIL (Dream): 1 (`perl`)
- MISSING_HINT: 19
- COMPOSE_FAIL: 0
- TIMEOUT: 0

Every row composed. Store `target` is the catalog name on all of them.

## Fix cluster

One bug class: **configure argv[0] is not the row’s language binary, and a miss uses the language install hint.**

| Row | Configure | `programs` | Same trap if language is installed |
|---|---|---|---|
| `perl` | `cpanm` | `perl` | **hit on this machine** |
| `ruby` | `bundle` | `ruby` | yes |
| `php` | `composer` | `php` | yes |
| `node` | `npm` | `node` | no (`npm` is here) |
| `crystal` | `shards` | `crystal`, `shards` | only if both missing; `shards` miss is still “Install Crystal” |

`bun` / `dart` / `cmake` / `elixir` configure the same binary they already list.

Honest fix: a missing configure/build helper is not “install the language.” Either list every exec’d binary in `programs` and hint the one that was missing, or skip configure when that helper is absent and the run program exists. For `perl` on this corpus, `cpanm` is also unnecessary — `cpanfile` has no deps and `limits.pl` is core-only.

## Other notes (not this smoke’s fail)

- 24 of 25 dests collapsed `limits.foo` / `label.foo` / `score.foo` into one file owned by `limits.foo`. Only `dart` kept per-unit files. Locking `score.foo` would not freeze the formula on the passing rows.
- `ensure_output_dirs` still creates wipe dirs (`target/`, `build/`, `.zig-cache`, …) before the missing-program check, so those empty dirs exist on every MISSING_HINT dest.
- `make` writes the binary as `./limits`, not under wipe `target/`.
