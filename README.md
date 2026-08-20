# Dream

Executable pseudocode. A programmer writes foocode in `.foo` files. Dream interprets that notation.

The product contract is the Dream vault: `MVP.md` and `Core Rules.md`.

## Phase 1

```bash
cp .env.example .env
# or put secrets in .env.local (overrides .env)

cargo run -- now examples/hey-you.foo
cargo run -- now --strict examples/hey-you.foo
```

## Config

```env
OPENAI_API_KEY=...
DREAM_MODEL=gpt-4.1
DREAM_TURN_CAP=10
```

## CLI

```bash
dream now [--strict] <file.foo>
```

Composition (`-t` / `-o`) is not implemented yet.
