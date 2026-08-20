# Dream

Executable pseudocode. A programmer writes foocode in `.foo` files. Dream interprets that notation.

The product contract is the [Dream vault](https://github.com/Spencer1O1/DreamVault): `MVP.md` and `Core Rules.md`. Implementation progress is [docs/plan.md](docs/plan.md).

```bash
cp .env.example .env
# or put secrets in .env.local (overrides .env)

cargo run -- now examples/hey-you.foo
cargo run -- now --strict examples/hey-you.foo

cargo run -- examples/hey-you.foo -t rust -o ./out
cargo run -- --strict examples/hey-you.foo -t rust -o ./out --run
```

`-o` replaces the whole folder after a successful compose. A failed compose leaves the destination alone. Compose prints each tool call on stderr (name and path, not file contents).

`--build` / `--run` exec the declared catalog toolchain in `-o`. A failed **build** may go back to the composer a bounded number of times. `--no-warn` treats toolchain warnings as a failed build (and thus repairable). A failed run, a missing toolchain, or `unsupported` does not repair. Dream does not install tools.

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
dream [--strict] [--no-warn] <file.foo> -t <target> -o <dir>
dream [--strict] [--no-warn] <file.foo> -t <target> -o <dir> --build
dream [--strict] [--no-warn] <file.foo> -t <target> -o <dir> --run
```
