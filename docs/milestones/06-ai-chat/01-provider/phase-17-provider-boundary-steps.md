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
3. ~~**Gemini request body**~~ — serde types, pure builder, JSON-shape test. **Done** — `49f7902`.
4. ~~**Gemini response body**~~ — serde types, pure reader, captured-JSON tests. **Done** — `f6a1c16`.
5. ~~**The HTTP call**~~ — `reqwest`, `#[ignore]` live test, iOS build check. **Done** — `2fa9bfe`.
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

> **Status:** done — committed in `49f7902` (143 tests green). `Content` and `Part` were
> made `pub(crate)` rather than private; both tests were written at commit time and the
> shape test verified live by mutation. The commit also carried a `pollster` dev-dep bump
> to 1 and a re-indent of `Cargo.toml`. Clippy reports only the expected dead-code warnings
> until the UI wires `ai` in.

---

## Step 4 — Gemini response body

**What it is.** The second half of the translation: Gemini's `generateContent` JSON →
`Result<Reply, ChatError>`. Still no network — `reqwest` in Step 5 will hand us a body as
text, and this step decides what that text *means*. A success looks like this (only the
fields we read, plus the ones we must be able to ignore):

```json
{
  "candidates": [
    {
      "content": { "role": "model", "parts": [ { "text": "Ankh-Morpork" } ] },
      "finishReason": "STOP"
    }
  ],
  "usageMetadata": { "promptTokenCount": 9, "candidatesTokenCount": 3 }
}
```

And a prompt the safety filter refused arrives as **HTTP 200 with no candidates**:

```json
{ "promptFeedback": { "blockReason": "SAFETY" } }
```

That second shape is the reason this step exists as its own step. A 200 is not a reply;
"no candidates" is the empty case, and it has to become `ChatError::Empty` here, in pure
Rust, not somewhere in the reader when it finds an empty string.

**Check (`cargo test ai::gemini`)** — pure Rust, `#[test]`, appended to the existing test
module in `src/ai/gemini.rs`:

```rust
#[test]
fn a_captured_success_becomes_a_reply() {
    let body = r#"{
        "candidates": [
            {
                "content": { "role": "model", "parts": [ { "text": "Ankh-Morpork" } ] },
                "finishReason": "STOP"
            }
        ],
        "usageMetadata": { "promptTokenCount": 9, "candidatesTokenCount": 3 }
    }"#;

    let reply = reply_from(serde_json::from_str(body).unwrap()).unwrap();

    assert_eq!(reply.text, "Ankh-Morpork");
}

#[test]
fn several_text_parts_are_joined_into_one_reply() {
    let body = r#"{ "candidates": [ { "content": { "parts": [
        { "text": "Ankh-" }, { "text": "Morpork" }
    ] } } ] }"#;

    let reply = reply_from(serde_json::from_str(body).unwrap()).unwrap();

    assert_eq!(reply.text, "Ankh-Morpork");
}

#[test]
fn a_blocked_prompt_has_no_candidates_and_is_empty() {
    let body = r#"{ "promptFeedback": { "blockReason": "SAFETY" } }"#;

    let result = reply_from(serde_json::from_str(body).unwrap());

    assert!(matches!(result, Err(ChatError::Empty)), "{result:?}");
}

#[test]
fn a_candidate_with_no_text_is_empty_too() {
    let body = r#"{ "candidates": [ { "content": { "parts": [ {} ] } } ] }"#;

    let result = reply_from(serde_json::from_str(body).unwrap());

    assert!(matches!(result, Err(ChatError::Empty)), "{result:?}");
}
```

Watch all four fail (no `GenerateResponse`, no `reply_from`), then make them pass. Note
what the bodies deliberately leave out: `role` and `finishReason` in the second and fourth,
`candidates` entirely in the third. Every one of those is a field Gemini can omit or that
we never read, and the tests pin that the parser survives their absence. The first test
carries the fields we *don't* model (`finishReason`, `usageMetadata`) so it also pins that
unknown keys are ignored.

**Minimal implementation** — in `src/ai/gemini.rs`, next to the request types:

```rust
use serde::{Deserialize, Serialize};

use super::{ChatError, Message, Reply, Role};

#[derive(Debug, Deserialize)]
pub(crate) struct GenerateResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    #[serde(default)]
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

pub(crate) fn reply_from(response: GenerateResponse) -> Result<Reply, ChatError> {
    let text: String = response
        .candidates
        .into_iter()
        .next()
        … flatten the first candidate's parts, keep the `Some(text)`s, concatenate …;

    if text.is_empty() { Err(ChatError::Empty) } else { Ok(Reply { text }) }
}
```

Fill in the `…`. One clean shape is `.into_iter().next().into_iter()` to turn
`Option<Candidate>` into a zero-or-one iterator, then `.flat_map(|c| c.content.parts)`,
`.filter_map(|p| p.text)`, `.collect::<String>()`. Another is a plain `match` on
`candidates.into_iter().next()` with an early `return Err(ChatError::Empty)`. Both are
fine; pick the one you can read back in a month.

Separate `ResponseContent`/`ResponsePart` from the request-side `Content`/`Part` rather than
deriving both `Serialize` and `Deserialize` on one type. They look alike today, but `role`
is required going out and optional coming back, `text` is required going out and optional
coming back — the two directions have different rules, and one struct would have to lie
about one of them.

**Why it works.** `#[derive(Deserialize)]` is the mirror of Step 3: the struct is the
schema, and `serde_json::from_str` walks the JSON against it. Two attributes do the real
work here. `#[serde(default)]` on a `Vec` field means "if the key is missing, use
`Vec::new()`" — without it, the blocked-prompt body would fail to parse with *"missing
field `candidates`"*, and a parse error is the wrong diagnosis for a safety block. Serde
ignores unknown keys by default (the opposite, `#[serde(deny_unknown_fields)]`, is opt-in),
which is exactly what you want at an API boundary: Google adds fields all the time, and none
of them should break the reader.

`Option<String>` on `text` is the other half of the same idea. A `Part` is a tagged union on
the wire — it might be `text`, `inlineData`, `functionCall` — and a missing `text` key
deserializes to `None` for free because serde treats `Option` fields as optional. So the
types encode "we only understand text parts and skip the rest," and the code never has to
say it.

`reply_from` takes `GenerateResponse` **by value** and `Reply` takes ownership of the text
by moving it out of the parts: `into_iter()` everywhere, no `.clone()`. The response was
built for this one call and is dead after it, so consuming it is the honest signature. That
is also why `collect::<String>()` works — `String: FromIterator<String>`, so concatenating
owned strings is one line and one allocation.

`matches!` in the tests is the idiom for "is it this variant" when the error type does not
derive `PartialEq` (and `ChatError` does not — `thiserror` enums usually wrap non-comparable
sources later). It expands to a `match` that returns `bool`, so you get a structural check
without adding a derive just for tests.

**Scope note.** No HTTP status handling, no error JSON (`{ "error": { "code": 400, … } }`)
— those come *with* the status code in Step 5, where `ChatError` grows `Http` and `Api`
variants. `finishReason: "MAX_TOKENS"` is treated as a normal (truncated) reply for now; a
visible marker for that is a Phase 19 UI decision. `promptFeedback.blockReason` is dropped
on the floor here; if a later phase wants to tell the reader *why* it got nothing, add a
`Blocked(String)` variant then, with the captured body above as its test.

> **Status:** done — committed in `f6a1c16` (147 tests green). Built with the `let … else`
> early return; the four tests were written at commit time and the multi-part one verified
> live by mutation. Clippy still reports only dead-code warnings for the unwired `ai` module.

---

## Step 5 — The HTTP call

**What it is.** Join Steps 3 and 4 with the one piece of code that touches the socket: a
`Gemini` struct that holds the key, the model name, and a `reqwest::Client`, and an
`impl ChatProvider for Gemini` whose `complete` posts `request_body(messages)` and feeds the
answer through `reply_from`. The phase's exit criterion lives here — an `#[ignore]`d test
that, given `GEMINI_API_KEY`, gets a real sentence back through the *trait*, not through
`Gemini` directly.

The wire details, all of which stay in `gemini.rs`:

- `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
- key in the `x-goog-api-key` header (not the query string — it would end up in logs)
- default model `gemini-3.5-flash-lite` — the newest stable Flash-Lite with a free tier as of
  2026-09 (checked against the Gemini models and pricing pages); a field, so "Flash-Lite → Flash" is a settings
  change, never a code change

**Check** — two kinds this time.

*Pure, `cargo test ai::gemini`*, in the existing test module:

```rust
#[test]
fn the_endpoint_names_the_model_and_the_method() {
    let url = endpoint("gemini-3.5-flash-lite");

    assert_eq!(
        url,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash-lite:generateContent"
    );
}

#[test]
fn a_non_success_status_becomes_an_api_error() {
    let result = check_status(reqwest::StatusCode::BAD_REQUEST, r#"{"error":{"code":400}}"#.into());

    assert!(
        matches!(&result, Err(ChatError::Api { status: 400, body }) if body.contains("400")),
        "{result:?}"
    );
}
```

*Live, `cargo test ai::gemini -- --ignored`*, same module. It needs a tokio runtime (see
"why" below), so add to `[dev-dependencies]`:

```toml
tokio = { version = "1", features = ["rt", "macros"] }
```

```rust
#[tokio::test]
#[ignore = "needs GEMINI_API_KEY and the network"]
async fn a_real_gemini_answers_through_the_trait() {
    let key = std::env::var("GEMINI_API_KEY").expect("set GEMINI_API_KEY to run this");
    let gemini = Gemini::new(key);
    let messages = [Message::user("Reply with exactly one word: pong")];

    let reply = ChatProvider::complete(&gemini, &messages).await.unwrap();

    assert!(reply.text.to_lowercase().contains("pong"), "{reply:?}");
}
```

Run the pure tests first and watch them fail (no `endpoint`, no `check_status`, no `Api`
variant). Then `GEMINI_API_KEY=… cargo test ai::gemini -- --ignored` once you have the
implementation; that one run *is* the phase's acceptance test, so paste its output into
the commit body. Finally `dx build --platform ios` — the point is not the app, it is that
`reqwest` with `rustls` links for iOS at all.

**Minimal implementation.**

`Cargo.toml`, `[dependencies]` — the same features Dioxus already pulls, so the lockfile
does not grow:

```toml
reqwest = { version = "0.13", default-features = false, features = ["rustls-tls", "json"] }
```

`src/ai/mod.rs` — `ChatError` grows the two shapes an HTTP call can fail in:

```rust
#[derive(Debug, thiserror::Error)]
pub(crate) enum ChatError {
    #[error("the provider returned no answer")]
    Empty,
    #[error("could not reach the provider: {0}")]
    Http(#[from] reqwest::Error),
    #[error("the provider rejected the request ({status}): {body}")]
    Api { status: u16, body: String },
}
```

`src/ai/gemini.rs`:

```rust
use reqwest::StatusCode;

use super::{ChatError, ChatProvider, Message, Reply, Role};

const DEFAULT_MODEL: &str = "gemini-3.5-flash-lite";

pub(crate) struct Gemini {
    key: String,
    model: String,
    client: reqwest::Client,
}

impl Gemini {
    pub(crate) fn new(key: String) -> Self {
        Gemini { key, model: DEFAULT_MODEL.to_string(), client: reqwest::Client::new() }
    }
}

fn endpoint(model: &str) -> String {
    format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent")
}

fn check_status(status: StatusCode, body: String) -> Result<String, ChatError> {
    if status.is_success() { Ok(body) } else { Err(ChatError::Api { status: status.as_u16(), body }) }
}

impl ChatProvider for Gemini {
    async fn complete(&self, messages: &[Message]) -> Result<Reply, ChatError> {
        let response = self
            .client
            .post(endpoint(&self.model))
            .header("x-goog-api-key", &self.key)
            .json(&request_body(messages))
            .send()
            .await?;

        let status = response.status();
        let body = check_status(status, response.text().await?)?;
        let parsed: GenerateResponse = serde_json::from_str(&body)?;
        reply_from(parsed)
    }
}
```

The `serde_json::from_str(&body)?` needs one more `#[from]` on `ChatError` — either a
`Json(#[from] serde_json::Error)` variant, or fold it into `Api` by hand with
`.map_err(…)`. Pick one and say why in the commit. (Reading the body as `text()` and
parsing it yourself, instead of `response.json()`, is deliberate: on a non-2xx the body is
the error message you want to *show*, and `.json::<GenerateResponse>()` would throw it
away as a deserialize failure.)

**Why it works.** `async fn complete` in the trait now does real suspension: every `.await`
on a `reqwest` future hands control back to the executor until the socket has something.
That is why the ignored test cannot use `pollster` — `reqwest` is built on `hyper`, which
registers its sockets with **tokio's reactor**, and `pollster::block_on` polls on a bare
thread with no reactor to wake it. The call would panic with *"there is no reactor
running"*. `#[tokio::test]` spins up a single-threaded runtime for that one test; in the
app, Dioxus already runs tokio, so `complete` needs nothing extra there.

`#[from]` on a variant is `thiserror` generating `impl From<reqwest::Error> for ChatError`,
which is what makes `?` work: `?` calls `From::from` on the error before returning it. So
one attribute replaces a `.map_err(ChatError::Http)` at every `.await`. `Api` has no
`#[from]` because nothing produces it but us — it is the "the server answered, and the
answer was no" case, which is not a transport error and should read differently in the UI.

`reqwest::Client` is an `Arc` around a connection pool. Building one per request would
re-do the TLS handshake every time; building one per `Gemini` and holding it in the struct
is the documented pattern, and it is `Clone` for the same reason. `&self.key` in the header
call borrows — `reqwest` copies it into the request, so the struct stays untouched and
`complete` can keep taking `&self`.

`endpoint` and `check_status` are split out for exactly one reason: they are the only parts
of `complete` that a test can reach without a server, and both encode a decision (the URL
shape, "which statuses are failures") that would otherwise be untested. `status.is_success()`
is the 2xx range — `200` and `204` both count, `3xx` does not, which is right because
`reqwest` follows redirects itself.

**Scope note.** No timeout, no retry, no cancellation — Phase 22. The key is a plain
`String` in memory until Phase 18 puts it in the keychain. The model is not yet
configurable from outside (`new` takes only the key); a `with_model` builder is a Step 6
consideration once the settings row in Phase 18 says what it needs. No `systemInstruction`
— the prompt template already puts context in the first user turn.

> **Status:** done — committed in `2fa9bfe` (149 tests green, 1 ignored; the ignored live test
> passed with `GEMINI_API_KEY` in 0.72s, and `dx build --platform ios` linked clean). The
> parse error got its own `Json` variant. `reqwest` was first pinned at 0.13, which pulled
> a second copy beside the 0.12 Dioxus already uses, and was dropped back to 0.12. The three
> tests were written at commit time; the status one was verified live by mutation.

---

## Step 6 — Review and refactor

**What it is.** The phase closes by stepping back over `src/ai/` with working code in
hand. Nothing here changes behaviour; the existing 149 tests are the spec, and clippy going
from 24 warnings to 0 is the target. Work the list top to bottom, running `cargo test` after
each item, and tick as you go.

**Check (`cargo test && cargo clippy --all-targets`)** — the safety net, not a target.
Before you start: 149 passed, 1 ignored, 24 dead-code warnings. After: the same 149 passed,
1 ignored, **0 warnings** (the `block v0.1.6` future-incompat note is upstream, not ours).
One new test is fair game, under item 5, because that refactor makes a lifetime relationship
explicit that was implicit before. Nothing else gets a new test — if you find yourself
needing one, the change is a feature, not a refactor.

### Punch-list

- [x] **1. Silence the dead-code storm honestly.** `src/main.rs`: `#[allow(dead_code)] mod ai;`
  — with a comment saying *until Phase 19 wires the chat panel*. Step 1 planned exactly this.
  A module-level `allow` is one line to remove later; twenty-four `#[allow]`s sprinkled over
  items would each need remembering. Do not "fix" the warnings by making everything `pub` —
  that trades an honest "unused yet" for a dishonest "used elsewhere".

- [x] **2. Tighten `gemini.rs`'s public surface to `Gemini` and `Gemini::new`.** Everything
  else — `GenerateRequest`, `Content`, `Part`, `GenerateResponse`, `request_body`,
  `reply_from` — drop the `pub(crate)`. Nothing outside `ai::gemini` names them; the Step 5
  live test lives *inside* the module, so the reason the Step 3 doc gave for `pub(crate)`
  never materialised. The payoff is the crux of the phase made checkable: `grep pub gemini.rs`
  now lists the whole boundary, and the compiler will refuse the first `use ai::gemini::Part`
  someone writes in the reader.

- [x] **3. Put the file in reading order.** `gemini.rs` currently opens with `Gemini` and
  `complete`, which call `request_body` and `reply_from` defined below them. Reorder to:
  request types + `request_body` → response types + `reply_from` → `Gemini`, `endpoint`,
  `check_status`, `impl ChatProvider`. Rust does not care about item order; readers do —
  the file should tell the Step 3 → 4 → 5 story top to bottom, and `complete` should read as
  the join of two things you have already seen. While there, swap `use crate::ai::{…}` for
  `use super::{…}`: it says "I am a child of `ai`" instead of restating the path, and it
  survives the module being moved.

- [x] **4. `MAX_MESSAGE_CHARS` → `MAX_PASSAGE_CHARS`.** It caps the *quoted passage*, not the
  message — the title and author are added on top. The name is the only place the mistake
  can hide, because nothing else about the cap is wrong. The test that names it moves too.

- [x] **5. Borrow in the request instead of cloning.** The Step 3 doc deferred this until
  Step 5 showed whether the request outlives the messages. It does not: `complete` builds
  `request_body(messages)` and serializes it with `.json(&…)` on the same line, and the
  `GenerateRequest` is dropped before `.send()`. So:

  ```rust
  struct GenerateRequest<'a> { contents: Vec<Content<'a>> }
  struct Content<'a> { role: &'static str, parts: Vec<Part<'a>> }
  struct Part<'a> { text: &'a str }

  fn request_body(messages: &[Message]) -> GenerateRequest<'_> { … }

  impl<'a> From<&'a Message> for Content<'a> {
      fn from(message: &'a Message) -> Self {
          Content { role: …, parts: vec![Part { text: &message.text }] }
      }
  }
  ```

  Three structs grow a `'a`, one `clone()` disappears, and `serde` is happy because
  `&str: Serialize`. The `'_` in `request_body`'s return type is lifetime elision: "the
  same lifetime as the one input reference." The `impl<'a> From<&'a Message>` is where the
  lesson is — `From<&Message>` with an elided lifetime cannot express "the output borrows
  from the input", so the lifetime has to be named on the `impl`. If a later step ever needs
  the request to outlive the messages, the compiler will say so at the call site, which is
  exactly the guarantee you want from a borrow. *One new test is allowed here*: none is
  needed, the existing shape tests cover it — skip it unless the borrow checker teaches you
  something worth pinning.

- [x] **6. `write!` instead of `push_str(&format!(…))` in `prompt.rs`.** Three places.
  `format!` allocates a temporary `String` that `push_str` immediately copies and drops;
  `write!(out, " by {author}")` (with `use std::fmt::Write;`) formats straight into `out`.
  It returns a `fmt::Result` that is always `Ok` for a `String`, so either `let _ =` it or
  `.expect("writing to a String cannot fail")` — the second documents *why* it is safe. The
  first `format!` that seeds `out` stays; it is the allocation you want.

- [x] **7. One voice for `ChatError` messages.** Three variants say "the provider …", one says
  "could not read provider answer". Make it "could not read the provider's answer: {0}".
  These strings are the UI copy in Phase 19 until someone writes better; they should already
  read as one sentence-set.

**Not on the list, and why.** `Message`'s fields stay private while `Reply.text` is
`pub(crate)` — asymmetric, but Phase 19 is the first code that needs to *read* a `Message`
(to render history), and it should decide between accessors and `pub(crate)` fields with a
real caller in front of it. `Gemini::with_model` waits for Phase 18's settings row for the
same reason. `check_status` taking `String` rather than `&str` is right: the body is moved
into the error, not copied.

**Why the pass matters.** Items 2 and 5 are the ones that change what the code *promises*.
After 2, the provider boundary the phase was named for is enforced by visibility, not by
convention; after 5, the type signature of the request says "borrowed from the conversation"
and the compiler polices it. The rest is legibility — but this file is the template the
second provider will be written from, so its shape is worth getting right once.

> **Written by:** `lbb:next-implement` — the seven punch-list edits were applied by the agent,
> reviewed by hand. The `simplify` pass proposed reverting items 5 and 6 as "harder for a
> learner"; both were kept deliberately, since teaching the lifetime and `fmt::Write` idioms is
> the point of the pass. The `#[allow(dead_code)]` landed without the planned comment, per the
> no-agent-comments rule — add it by hand.

> **Status:** done — committed in `2b9e826` (149 tests green, 1 ignored; clippy clean).
