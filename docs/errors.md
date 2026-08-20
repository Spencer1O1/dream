# Errors

Process abort prints the subtype. Tool refusals use the detail only.

| Type | When |
|------|------|
| `UsageError` | This invocation does not apply: flags, not a `.foo`, dest occupied, lock with no store. |
| `ConfigError` | Dream cannot start: environment or settings. |
| `InterpreterError` | The dreamed program stopped (`dream_error`, lucid turn cap). Not compose. |
| `ComposerError` | Composition, lock, repair, or build stopped. |
| `RuntimeError` | Host plumbing that is neither the program nor compose: OpenAI, I/O, JSON, a corrupt store. Shared helpers that do not know the session. |

`dream_error` is always `InterpreterError`. The compose turn cap is `ComposerError`.

A drifted lock is `ComposerError` on both `dream lock` and compose `check`. Same class as a missing locked artifact. Usage stays for “this command does not apply.”
