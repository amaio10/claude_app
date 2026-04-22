# Test note — Obsidian rendering

Check headings, body copy, **bold**, *italic*, ~~strikethrough~~, `inline code`.

## Math — KaTeX

Inline: the famous $E = mc^2$ and Euler's $e^{i\pi} + 1 = 0$.

Block:

$$
\mathcal{L} = -\frac{1}{4} F_{\mu\nu} F^{\mu\nu} + i\bar{\psi}\gamma^\mu D_\mu \psi - m\bar{\psi}\psi
$$

$$
\int_{-\infty}^{\infty} e^{-x^2} \, dx = \sqrt{\pi}
$$

## Code — Shiki

```rust
fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

```ts
type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

const safeParse = <T>(raw: string): Result<T, Error> => {
    try {
        return { ok: true, value: JSON.parse(raw) };
    } catch (e) {
        return { ok: false, error: e as Error };
    }
};
```

```python
import numpy as np

def softmax(x):
    ex = np.exp(x - x.max())
    return ex / ex.sum(axis=-1, keepdims=True)
```

## Callouts — Obsidian

> [!note] Small reminder
> Callouts support **markdown** inside their body.

> [!tip]
> No title — kind only.

> [!warning] Watch out
> Use `cargo run` only from the backend directory.

> [!success] Ship
> Multi-session PTY works, LaTeX renders, design is clean.

> [!danger] Critical
> Don't commit `.env`.

> [!question] FAQ
> Why Groq? Fast, free, same Whisper model as OpenAI.

## Lists & tables

- [x] Backend axum
- [x] Voice-to-text
- [x] Multi-session
- [ ] Tauri packaging

| Feature       | Status  | Notes            |
|---------------|---------|------------------|
| Whisper       | ✅      | Groq, 300ms       |
| Claude stream | ✅      | WS /ws            |
| PTY replay    | ✅      | 64KB ring buffer  |
| Markdown      | 🚧      | You're reading it |

## Quote

> Simplicity is the ultimate sophistication.
> — Leonardo da Vinci

---

End of test.
