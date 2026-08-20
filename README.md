# Dream

Executable pseudocode. A programmer writes foocode in `.foo` files. Dream interprets that notation.

The product contract is the Dream vault: `MVP.md` and `Core Rules.md`.

## Phase 1

```bash
cp .env.example .env
# or put secrets in .env.local (overrides .env)

cargo run -- now examples/hello.foo
cargo run -- now --strict examples/hello.foo
```

## Config

```env
OPENAI_API_KEY=...
DREAM_MODEL=gpt-4.1
```

## CLI

```bash
dream now [--strict] <file.foo>
```

Composition (`-t` / `-o`) is not implemented yet.
