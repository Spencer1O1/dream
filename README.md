# Dream

Executable pseudocode. A programmer writes foocode in `.foo` files. Dream interprets that notation.

The product contract is the Dream vault: `MVP.md` and `Core Rules.md`.

## Phase 1–3

```bash
cp .env.example .env
# or put secrets in .env.local (overrides .env)

cargo run -- now examples/hey-you.foo
cargo run -- now --strict examples/hey-you.foo

cargo run -- examples/hello.foo -t rust -o ./out
cargo run -- --strict examples/hello.foo -t rust -o ./out
```

`-o` replaces the whole folder after a successful compose. A failed compose leaves the destination alone. `--build` and `--run` are not implemented yet.

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
```
