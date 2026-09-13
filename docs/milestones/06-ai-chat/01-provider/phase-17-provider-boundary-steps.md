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

1. ~~**The vocabulary and the trait**~~ — types + `ChatProvider` + a test `Fake`. **Done** — `aa29912`.
2. ~~**The prompt template**~~ — book, chapter, selection → a draft for the input box, with a cap. **Done** — `e74b4ed`.
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

> **Status:** done — committed in `aa29912` (137 tests green). The test was written at
> commit time and verified live by mutation; the pre-existing page-label failure was
> re-pinned in `9005f51` first.

---

## Step 2 — The prompt template

**What it is.** The feature spec says the chat input opens *prefilled* — book, author, the
passage — and the reader types their question after it. So the template's output is a
**draft string for the input box**, not a finished message: `prompt::draft(&Passage) ->
String`. It is sent as `Message::user(draft)` only once the user hits send. That also
settles the "no `System` role" question from Step 1: the context rides inside the first user
turn, which every provider understands.

**Check (`cargo test ai::prompt`)** — pure Rust, `#[test]`, in `src/ai/prompt.rs`:

```rust
#[cfg(test)]
mod test {
    use super::*;

    fn passage<'a>(text: &'a str) -> Passage<'a> {
        Passage {
            title: "The Colour of Magic",
            author: Some("Terry Pratchett"),
            chapter: Some("The Colour of Magic"),
            text,
        }
    }

    #[test]
    fn the_draft_names_the_book_and_quotes_the_passage() {
        let draft = draft(&passage("Rincewind ran."));

        assert!(draft.contains("The Colour of Magic"), "{draft}");
        assert!(draft.contains("Terry Pratchett"), "{draft}");
        assert!(draft.contains("> Rincewind ran."), "{draft}");
        assert!(draft.ends_with("\n\n"), "the cursor lands on a fresh line: {draft:?}");
    }

    #[test]
    fn missing_author_and_chapter_leave_no_holes() {
        let draft = draft(&Passage {
            title: "Anonymous",
            author: None,
            chapter: None,
            text: "x",
        });

        assert!(!draft.contains("by "), "{draft}");
        assert!(!draft.contains("chapter"), "{draft}");
    }

    #[test]
    fn a_long_passage_is_cut_on_a_char_boundary() {
        let long = "é".repeat(MAX_PASSAGE_CHARS + 10);

        let draft = draft(&passage(&long));

        let quoted = draft.split("> ").nth(1).unwrap();
        assert_eq!(quoted.chars().filter(|c| *c == 'é').count(), MAX_PASSAGE_CHARS);
        assert!(quoted.contains('…'), "truncation is visible: {quoted}");
    }
}
```

Watch it fail (no `prompt` module), then make it pass. The third test is the important one:
`"é"` is two bytes, so a byte-indexed slice at the cap would panic mid-character.

**Minimal implementation** — `pub(crate) mod prompt;` in `src/ai/mod.rs`, and
`src/ai/prompt.rs`:

```rust
pub(crate) const MAX_PASSAGE_CHARS: usize = 4_000;

pub(crate) struct Passage<'a> {
    pub(crate) title: &'a str,
    pub(crate) author: Option<&'a str>,
    pub(crate) chapter: Option<&'a str>,
    pub(crate) text: &'a str,
}

pub(crate) fn draft(passage: &Passage) -> String {
    let mut out = format!("I'm reading *{}*", passage.title);
    if let Some(author) = passage.author { … push " by {author}" }
    if let Some(chapter) = passage.chapter { … push ", chapter \"{chapter}\"" }
    out.push_str(".\n\n");
    for line in clipped(passage.text).lines() { … push "> {line}\n" }
    out.push('\n');
    out
}

fn clipped(text: &str) -> Cow<'_, str> {
    match text.char_indices().nth(MAX_PASSAGE_CHARS) {
        None => Cow::Borrowed(text),
        Some((end, _)) => Cow::Owned(format!("{}…", &text[..end])),
    }
}
```

Fill in the `…` yourself; the shape is the point, not the exact wording — adjust the
prose until the first test's `contains` checks match what you actually write. Make sure a
multi-line selection quotes every line (the `for line in …lines()` loop), so a passage
spanning paragraphs still reads as one blockquote.

**Why it works.** `Passage<'a>` borrows everything. The caller (the reader, in Phase 20)
already owns a `Book { title: String, author: Option<String> }` and a chapter label, and it
would be wasteful to clone four strings just to format them once. The `'a` says: every
`&str` in here lives at least as long as the `Passage` — one lifetime, because they all come
from the same caller frame. `Option<&'a str>` is the borrowed twin of `Option<String>`, and
`book.author.as_deref()` is the one-call conversion at the call site.

`char_indices().nth(N)` is the correct "N characters in" cursor: it walks UTF-8 boundaries
and returns the *byte* offset where the (N+1)th char starts, which is exactly the safe slice
end. `None` means the text was shorter than the cap. `Cow` lets the common case — a short
selection — return the borrowed input with no allocation, while the truncated case owns its
new string; the caller treats both as `&str` via deref.

`format!` and `push_str` on one growing `String` is the idiomatic way to assemble a
multi-part message; `String` is a `Vec<u8>` underneath, so appends are amortised O(1) and
there is no intermediate `Vec<String>` + `join`.

**Scope note.** Nothing reads the selection out of the WebView yet — that is Phase 20. The
cap is by *characters*, not tokens; a token-aware cap is provider-specific and belongs in a
provider, if ever. The UI is not told when truncation happened beyond the visible `…`.

> **Status:** done — committed in `e74b4ed` (141 tests green). Built as `draft(&Passage) -> String`
> with `MAX_MESSAGE_CHARS` and an ASCII `...` marker; the four tests were written at commit
> time and the char-boundary one verified live by mutation. One clippy nit
> (`push_str("\n")` → `push('\n')`) was fixed before the commit.

---

## Step 3 — Gemini request body

**What it is.** The first half of the translation at the boundary: `&[Message]` in the
app's vocabulary → the JSON body Gemini's `generateContent` endpoint expects. No network,
no key, no `reqwest` — just `serde` types and one pure function, so the wire format has a
test that runs offline. The shape Gemini wants (only the fields we use):

```json
{
  "contents": [
    { "role": "user",  "parts": [ { "text": "Which city?" } ] },
    { "role": "model", "parts": [ { "text": "Ankh-Morpork" } ] }
  ]
}
```

Two things to notice: Gemini says `"model"` where we say `Assistant`, and every turn is a
list of `parts` even when there is one text part. Both are exactly the kind of detail that
must stay inside `gemini.rs`.

**Check (`cargo test ai::gemini`)** — pure Rust, `#[test]`, in `src/ai/gemini.rs`. First
add direct dependencies (both are already in the lockfile via Dioxus, so nothing new is
downloaded):

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Then the tests:

```rust
#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_conversation_becomes_gemini_contents() {
        let messages = [Message::user("Which city?"), Message::assistant("Ankh-Morpork")];

        let body = serde_json::to_value(request_body(&messages)).unwrap();

        assert_eq!(
            body,
            json!({
                "contents": [
                    { "role": "user",  "parts": [{ "text": "Which city?" }] },
                    { "role": "model", "parts": [{ "text": "Ankh-Morpork" }] },
                ]
            })
        );
    }

    #[test]
    fn an_assistant_turn_is_named_model_on_the_wire() {
        let body = serde_json::to_string(&request_body(&[Message::assistant("hi")])).unwrap();

        assert!(body.contains(r#""role":"model""#), "{body}");
        assert!(!body.contains("assistant"), "{body}");
    }
}
```

Watch it fail (no `gemini` module), then make it pass. Comparing `serde_json::Value`s in
the first test is the trick: it checks the *whole* shape at once, ignores key order and
whitespace, and prints a readable diff on failure. The second test pins the one rename that
would silently break a request.

**Minimal implementation** — `pub(crate) mod gemini;` in `src/ai/mod.rs`, and
`src/ai/gemini.rs`:

```rust
use serde::Serialize;

use super::{Message, Role};

#[derive(Debug, Serialize)]
pub(crate) struct GenerateRequest {
    contents: Vec<Content>,
}

#[derive(Debug, Serialize)]
struct Content {
    role: &'static str,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

pub(crate) fn request_body(messages: &[Message]) -> GenerateRequest {
    GenerateRequest {
        contents: messages.iter().map(Content::from).collect(),
    }
}

impl From<&Message> for Content {
    fn from(message: &Message) -> Self {
        Content {
            role: match message.role { … "user" / "model" },
            parts: vec![Part { text: message.text.clone() }],
        }
    }
}
```

Fill in the `match`. `Message`'s fields are private to `ai`, but `gemini` is a *child*
module of `ai`, so it can read them — Rust privacy is "visible in the defining module and
everything nested inside it." That is the whole reason the provider lives under `ai/` and
not beside it. If you would rather not lean on that, add `pub(crate)` to the fields; either
is defensible, but note which you chose.

**Why it works.** `#[derive(Serialize)]` generates the `serde::Serialize` impl from the
struct's shape: field names become keys, `Vec` becomes an array, nested structs nest. So
the *types* are the schema — there is no hand-written JSON string to drift. `&'static str`
for `role` is enough because the only two values are literals baked into the binary;
`String` for `text` because it is copied out of a `Message` the caller still owns (a
`&'a str` would work too and save the clone — a fair Step 6 refactor once you see whether
the request outlives the messages in Step 5).

`impl From<&Message> for Content` is the idiomatic home for a one-way conversion; it makes
`.map(Content::from)` read as a sentence and keeps the `match` on `Role` in one place. The
`&` matters: `iter()` yields `&Message`, and we do not want to consume the caller's slice.

`serde_json::to_value` versus `to_string`: `to_value` gives you a tree you can compare
structurally, `to_string` gives you bytes you can `contains`. Use the tree for shape
assertions and the string only when the exact spelling on the wire is the point.

**Scope note.** No `systemInstruction`, no `generationConfig`, no model name or URL — those
are Step 5 concerns (they live in the HTTP call, not the body). The response side is Step 4.
`GenerateRequest` is `pub(crate)` only so the ignored test in Step 5 can inspect it; it is
never imported outside `ai`.
