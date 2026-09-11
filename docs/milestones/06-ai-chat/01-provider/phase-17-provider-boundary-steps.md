# Phase 17 — Provider boundary + Gemini — build log

[← Phase doc](phase-17-provider-boundary.md)

Per-step check → minimal code → why, appended newest-last. The
[phase doc](phase-17-provider-boundary.md)'s "Planned steps" checklist is the high-level
index; this file is the detail and the build log.

Baseline when the phase opened: **135 tests green, 1 pre-existing failure**
(`ui::reader::test::the_page_label_waits_for_a_real_count`, unrelated to this phase — it
expects `"Page …"` and gets `"…"`). Fix or re-pin it before the first `lbb:commit`.

## The crux

**The provider is a boundary, and the thing on the app's side of it must be boring.** The
reader speaks `Message { role, text }`; each provider translates that into its own wire
format in its own file. Tests reach everything but the socket: request-building and
response-reading are pure functions over `serde` types, and the `reqwest` call that joins
them is the only thing behind `#[ignore]`.

## Step plan

1. **The vocabulary and the trait** — types + `ChatProvider` + a test `Fake`.
2. **The prompt template** — book, chapter, selection → opening messages, with a cap.
3. **Gemini request body** — serde types, pure builder, JSON-shape test.
4. **Gemini response body** — serde types, pure reader, captured-JSON tests.
5. **The HTTP call** — `reqwest`, `#[ignore]` live test, iOS build check.
6. **Review and refactor** — punch-list, suite green, clippy clean.

---

## Step 1 — The vocabulary and the trait

**Check (`cargo test ai::`)** — pure Rust, `#[test]`. Add to `Cargo.toml`
`[dev-dependencies]`:

```toml
pollster = "0.4"
```

Then in `src/ai/mod.rs`, in a `#[cfg(test)] mod test`:

```rust
use super::*;
use std::cell::RefCell;

struct Fake {
    reply: &'static str,
    seen: RefCell<Vec<Message>>,
}

impl ChatProvider for Fake {
    async fn complete(&self, messages: &[Message]) -> Result<Reply, ChatError> {
        self.seen.borrow_mut().extend_from_slice(messages);
        Ok(Reply { text: self.reply.to_string() })
    }
}

#[test]
fn a_provider_receives_the_conversation_and_answers() {
    let fake = Fake { reply: "Ankh-Morpork", seen: RefCell::new(Vec::new()) };
    let question = Message::user("Which city?");

    let reply = pollster::block_on(fake.complete(std::slice::from_ref(&question))).unwrap();

    assert_eq!(reply.text, "Ankh-Morpork");
    assert_eq!(fake.seen.borrow().as_slice(), &[question]);
}
```

Watch it fail to compile (no `ai` module, no types), then make it pass.

**Minimal implementation** — `mod ai;` in `src/main.rs`, and `src/ai/mod.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
    pub(crate) role: Role,
    pub(crate) text: String,
}

impl Message {
    pub(crate) fn user(text: impl Into<String>) -> Self { … }
    pub(crate) fn assistant(text: impl Into<String>) -> Self { … }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reply {
    pub(crate) text: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ChatError {
    #[error("the provider returned no answer")]
    Empty,
}

pub(crate) trait ChatProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Reply, ChatError>;
}
```

`ChatError` will grow `Http` and `Api` variants in Step 5; one variant is enough to give the
`Result` its shape now. Clippy will warn about dead code until Phase 19 wires the UI — a
`#[allow(dead_code)]` on the module is honest for the duration of the phase; remove it in
Step 6 if the warning is gone, or at Phase 19.

**Why it works.** `async fn` in a trait is sugar for a method returning `impl Future`,
which is why it is stable (1.75+) and why it needs no crate — and also why `dyn ChatProvider`
does not compile: the future's concrete type differs per implementor, so the vtable cannot
name it. Generics (`fn ask<P: ChatProvider>(p: &P)`) sidestep that entirely, and the
in-memory `Fake` is the first thing to benefit: no HTTP, no runtime, no mocking library.

`pollster::block_on` is the smallest possible executor: it polls the future on the current
thread and parks until it resolves. That is all a test needs; `Fake::complete` never
actually suspends. The `RefCell` in `Fake` is the usual answer to "a `&self` method that
records something" — interior mutability keeps the trait signature at `&self`, which is what
a shared, long-lived provider wants.

`Message` derives `Clone` and `PartialEq` so a test can compare a whole conversation by
value; `Role` is `Copy` because two variants with no payload are a byte. `impl Into<String>`
on the constructors lets call sites pass `&str` or `String` without a `.to_string()` at each
one — a small convenience the template in Step 2 leans on heavily.

**Scope note.** No provider does anything yet. No `system` role: Gemini models a system
prompt as a separate field, Anthropic likewise, OpenAI as a message — so it is *not* part of
the shared vocabulary; Step 2 decides how the template expresses it without one. Cancellation
and streaming are Phase 22.
