# Try

The corpus is `examples/`. Each subdirectory is one Dream project (the parent of the entry `.foo`). Do not put unrelated programs in the same folder or `list_source_files` will show all of them.

Unit tests do not call the model. These commands do.

Need `.env` or `.env.local` with `OPENAI_API_KEY` for `--lucid` and compose. `lock` / `unlock` do not.

Use a fresh dest for compose so leftover `target/` or a mixed tree does not block you:

```bash
cargo run -- examples/multifile/multifile.foo -t rust -o ./try
```

## Corpus

| Path | What it is for |
|---|---|
| `hello/hello.foo` | `--lucid` print |
| `hey-you/hey-you.foo` | `--lucid` stdin; compose + `--run` |
| `multifile/` (`multifile.foo` + `utils.foo`) | multi-unit compose, project layer, locks |
| `limits/` (`limits.foo` → `label.foo` → `score.foo`) | toolchain smoke: three units, extra dest files, no stdin |
| `fun/fun.foo` | informal mutation / odd control flow |
| `funky/funky.foo` | nonsense the interpreter must refuse |

There is no golden target tree. Look at stderr tool lines, `-o/.dream/provenance.json`, and whether the program runs.

## Lucid

```bash
cargo run -- --lucid examples/hello/hello.foo
cargo run -- --lucid examples/hey-you/hey-you.foo
cargo run -- --lucid examples/funky/funky.foo
```

`funky.foo` should be an interpreter error, not a successful print.

## Compose, build, run

```bash
cargo run -- examples/hey-you/hey-you.foo -t rust -o ./try --run
```

`--run` implies `--build`. Type a name and an integer. Then:

```bash
cat ./try/.dream/provenance.json
ls ./try
```

`Cargo.toml` is project-owned setup. The composer writes it. It is not listed under a unit.

In-place again, no `--fresh`:

```bash
cargo run -- examples/hey-you/hey-you.foo -t rust -o ./try
```

Unmanaged files you add (`README.md`) stay. `--fresh` drops Dream-owned paths and locks, not that README.

## Multi-file + lock

```bash
cargo run -- examples/multifile/multifile.foo -t rust -o ./try --build
cargo run -- lock examples/multifile/utils.foo -t rust -o ./try
```

`provenance.json` should show `utils.foo` with `"locked": true` and a `source_hash`.

A later compose may still run. `list_source_files` marks `utils.foo` as locked; `read_source_file` does too. If the model asks to write that unit anyway, the tool returns `{ "ok": false, "warning": "…" }`, stderr prints the same warning, and the file stays frozen. `--run` uses the frozen file. That is success, not a crash.

Source changed (should fail **before** any model turn):

```bash
echo "changed" >> examples/multifile/utils.foo
cargo run -- examples/multifile/multifile.foo -t rust -o ./try
# restore the original `timesThreePlusTwo` body
```

Missing locked artifact (same: fail at open):

```bash
rm ./try/src/utils.rs
cargo run -- examples/multifile/multifile.foo -t rust -o ./try
# restore the file, or unlock, or --fresh
```

Unlock, then a normal compose may rewrite `utils.foo` artifacts:

```bash
cargo run -- unlock examples/multifile/utils.foo -t rust -o ./try
```

`--lucid` on the same tree still returns `{ path, source }` only. Locks are `-t`-specific.

```bash
cargo run -- --lucid examples/multifile/multifile.foo
```

## Occupied dest

```bash
mkdir -p ./occupied && echo keep > ./occupied/README.md
cargo run -- examples/hello/hello.foo -t rust -o ./occupied
# error: pass --fresh or use an empty directory
```
