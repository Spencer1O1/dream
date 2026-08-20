# Dream

Executable pseudocode. A programmer writes foocode in `.foo` files. Dream interprets that notation.

The product contract is the Dream vault: `MVP.md` and `Core Rules.md`. Implementation progress is [docs/plan.md](docs/plan.md).

```bash
cp .env.example .env
# or put secrets in .env.local (overrides .env)

cargo run -- now examples/hey-you.foo
cargo run -- now --strict examples/hey-you.foo

cargo run -- examples/hey-you.foo -t rust -o ./out
cargo run -- --strict examples/hey-you.foo -t rust -o ./out --run
```

`-o` replaces the whole folder after a successful compose. A failed compose leaves the destination alone.

`--build` / `--run` exec the declared catalog toolchain in `-o`. If the builder is `unsupported` or missing on the machine, the project is still there; Dream errors with an install hint and does not install tools.

## Config

```env
OPENAI_API_KEY=...
DREAM_MODEL=gpt-4.1
DREAM_TURN_CAP=10
```

## CLI

```bash
dream now [--strict] <file.foo>
dream [--strict] <file.foo> -t <target> -o <dir>
dream [--strict] <file.foo> -t <target> -o <dir> --build
dream [--strict] <file.foo> -t <target> -o <dir> --run
```
