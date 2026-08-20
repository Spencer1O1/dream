# Dream

Executable pseudocode. A programmer writes foocode in `.foo` files. Dream interprets that notation.

The implemented v0 contract is the Dream vault: `MVP.md` and `Core Rules.md`. Next contract: `Artifact Ownership.md`. Implementation progress is [docs/plan.md](docs/plan.md).

```bash
cp .env.example .env
# or put secrets in .env.local (overrides .env)

cargo run -- now examples/hey-you.foo
cargo run -- now --strict examples/hey-you.foo

cargo run -- examples/hey-you.foo -t rust -o ./out
cargo run -- --strict examples/hey-you.foo -t rust -o ./out --run
cargo run -- examples/hey-you.foo -t rust -o ./out --fresh
```

Compose writes **in place** under `-o`. Dream records which `.foo` unit owns which output files. Unknown files stay. If `-o` already has files and no provenance, pass `--fresh` or use an empty directory. `--fresh` drops Dream-owned files and recomposes; unmanaged files stay. Compose prints each tool call on stderr (name and path, not file contents).

`--build` / `--run` exec the declared catalog toolchain in `-o`. A failed **build** may go back to the composer a bounded number of times. Repair may only overwrite existing unlocked unit-owned files. `--no-warn` treats toolchain warnings as a failed build (and thus repairable). A failed run, a missing toolchain, or `unsupported` does not repair. Dream does not install tools.

## Config

```env
OPENAI_API_KEY=...
DREAM_MODEL=gpt-4.1
DREAM_TURN_CAP=10
DREAM_REPAIR_CAP=3
```

## CLI

```bash
dream now [--strict] <file.foo>
dream [--strict] [--no-warn] [--fresh] <file.foo> -t <target> -o <dir>
dream [--strict] [--no-warn] [--fresh] <file.foo> -t <target> -o <dir> --build
dream [--strict] [--no-warn] [--fresh] <file.foo> -t <target> -o <dir> --run
```
