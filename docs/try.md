# Try

The corpus is `examples/`. Unit tests do not call the model. These commands do.

Need `.env` or `.env.local` with `OPENAI_API_KEY` for `--lucid` and compose. `lock` / `unlock` do not.

Use a fresh dest for compose so leftover `target/` or a mixed tree does not block you:

```bash
cargo run -- examples/multifile.foo -t rust -o ./try
```

## Corpus

| File | What it is for |
|---|---|
| `hello.foo` | `--lucid` print |
| `hey-you.foo` | `--lucid` stdin; compose + `--run` |
| `multifile.foo` + `utils.foo` | multi-unit compose, project layer, locks |
| `fun.foo` | informal mutation / odd control flow |
| `funky.foo` | nonsense the interpreter must refuse |

There is no golden target tree. Look at stderr tool lines, `-o/.dream/provenance.json`, and whether the program runs.

## Lucid

```bash
cargo run -- --lucid examples/hello.foo
cargo run -- --lucid examples/hey-you.foo
cargo run -- --lucid examples/funky.foo
```

`funky.foo` should be an interpreter error, not a successful print.

## Compose, build, run

```bash
cargo run -- examples/hey-you.foo -t rust -o ./try --run
```

`--run` implies `--build`. Type a name and an integer. Then:

```bash
cat ./try/.dream/provenance.json
ls ./try
```

Dream should own `Cargo.toml`. Composer writes of that path should have been rejected (stderr would say to use `set_dependencies`).

In-place again, no `--fresh`:

```bash
cargo run -- examples/hey-you.foo -t rust -o ./try
```

Unmanaged files you add (`README.md`) stay. `--fresh` drops Dream-owned paths and locks, not that README.

## Multi-file + lock

```bash
cargo run -- examples/multifile.foo -t rust -o ./try --build
cargo run -- lock examples/utils.foo -t rust -o ./try
```

`provenance.json` should show `utils.foo` with `"locked": true` and a `source_hash`.

Source changed (should fail **before** any model turn):

```bash
echo "changed" >> examples/utils.foo
cargo run -- examples/multifile.foo -t rust -o ./try
# restore
git checkout -- examples/utils.foo
# if you are not in git: put the original `timesThreePlusTwo` body back
```

Missing locked artifact (same: fail at open):

```bash
rm ./try/src/utils.rs
cargo run -- examples/multifile.foo -t rust -o ./try
# restore the file, or unlock, or --fresh
```

Hand-edit stays. Compose may still run; it must not rewrite the locked file:

```bash
# put utils.rs back first if you deleted it
printf 'pub fn times_three_plus_two(x: f64) -> f64 { 3.0 * x + 2.0 }\n' > ./try/src/utils.rs
# then hand-edit a comment in that file and compose again
cargo run -- examples/multifile.foo -t rust -o ./try
```

Unlock, then a normal compose may rewrite `utils.foo` artifacts:

```bash
cargo run -- unlock examples/utils.foo -t rust -o ./try
```

`--lucid` on the same tree still returns `{ path, source }` only. Locks are `-t`-specific.

```bash
cargo run -- --lucid examples/multifile.foo
```

## Occupied dest

```bash
mkdir -p ./occupied && echo keep > ./occupied/README.md
cargo run -- examples/hello.foo -t rust -o ./occupied
# error: pass --fresh or use an empty directory
```
