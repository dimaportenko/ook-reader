# Phase 22 — Streaming replies — build log

[← Phase 22](phase-22-streaming.md) · the phase doc holds the goal, decisions and the step
index; this file holds the test → code → why for each step, newest at the bottom.

## The crux

A reply becomes a sequence over time. Two changes follow from that. On the wire, TCP chunks
don't line up with SSE events, so a byte buffer must hand back only complete `data:` lines.
In the conversation, `Status::Replying(String)` holds the partial text while the reply is in
progress, and each delta is one signal write. `Task::cancel` drops the in-flight response,
and that is the whole of *Stop*.

## Step plan

1. The reply types itself in — `Replying(String)`, `append`, the real answer fed back word by word.
2. Stop — `Conversation::stop`, a *Stop* button that cancels the task.
3. The SSE line buffer — `src/ai/sse.rs`, pure, under `#[test]`.
4. The real stream — `ChatProvider::stream` with `on_text`, Gemini `alt=sse`, the fake removed.
5. Review and refactor.

**Why this order.** Dependency order would start with the SSE parser and a trait change,
which means two steps with nothing new on screen. Instead, Step 1 builds the part the user
sees: the growing turn. It gets its deltas by chopping the answer `complete` already
returns, so the drawer shows streaming before any streaming code exists. Step 2 needs a
reply slow enough to stop, and the fake's sleep provides one. Step 3 breaks the rule on
purpose: the buffer is the hard idea, and a parser belongs under tests before real bytes go
through it. Step 4 is then a swap inside a handler that already works: real deltas replace
fake ones, and the fake goes away.

## Step 1 — The reply types itself in

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

**What it is.** The "..." placeholder in the drawer becomes the assistant's reply, growing
word by word. There is no streaming from Gemini yet. The whole answer still arrives from
`complete`, and the component feeds it back into the conversation in pieces with a 40 ms
pause between them. Everything past the fake is the real design: `Conversation` learns to
hold a partial reply and to grow it.

**Check first — `cargo test chat::`.** Add these to the `test` module in
`src/chat/mod.rs`, and change `asking_records_the_turn_and_waits` to expect
`Status::Replying(String::new())` in place of `Status::Waiting`:

```rust
#[test]
fn deltas_grow_the_reply_in_progress() {
    let mut chat = Conversation::default();
    chat.ask("Which city?");

    chat.append("Ankh-");
    chat.append("Morpork");

    assert_eq!(chat.status(), &Status::Replying("Ankh-Morpork".to_owned()));
    assert_eq!(
        chat.messages(),
        &[Message::user("Which city?")],
        "the partial is not a finished turn yet"
    );
}

#[test]
fn settling_a_streamed_reply_makes_one_assistant_turn() {
    let mut chat = Conversation::default();
    chat.ask("Which city?");
    chat.append("Ankh-Morpork");

    chat.settle(reply("Ankh-Morpork"));

    assert_eq!(
        chat.messages(),
        &[Message::user("Which city?"), Message::assistant("Ankh-Morpork")]
    );
    assert_eq!(chat.status(), &Status::Idle);
}

#[test]
fn a_delta_with_no_reply_in_progress_is_ignored() {
    let mut chat = Conversation::default();

    chat.append("stray");

    assert_eq!(chat.status(), &Status::Idle);
}

#[test]
fn asking_while_replying_is_refused() {
    let mut chat = Conversation::default();
    chat.ask("First?");
    chat.append("Half an ans");

    assert!(!chat.ask("Second?"));

    assert_eq!(chat.messages().len(), 1);
}
```

`asking_while_replying_is_refused` replaces `asking_while_waiting_is_refused`. Its name
pointed at a variant that no longer exists, and it covered a subset of the new test. Its
`messages().len()` assertion moves across.

It fails to compile at first (`Replying` and `append` don't exist). That counts as red.

**Then the eyeball — `dx serve`.** Open a book, open the chat, ask something that needs a
paragraph ("Summarise this chapter's themes in five sentences"):

1. A "..." bubble appears while Gemini thinks, as before.
2. Then the bubble fills in word by word, left to right, and the list scrolls if it needs
   to.
3. When it finishes, the bubble looks exactly like any other assistant turn, and Reset and
   Send work again.
4. `cargo clippy` is clean.

**Minimal code.**

In `src/chat/mod.rs`, the variant carries the text so far:

```rust
pub(crate) enum Status {
    #[default]
    Idle,
    Replying(String),
    Failed(String),
}
```

`ask` sets `Status::Replying(String::new())`. Its guard can't use `==` against a single
value any more, because any `Replying(_)` must refuse. `matches!(self.status,
Status::Replying(_))` is the tool for that. Then:

```rust
pub(crate) fn append(&mut self, delta: &str) {
    if let Status::Replying(text) = &mut self.status {
        text.push_str(delta);
    }
}
```

`settle` stays as it is. It overwrites the status and pushes `reply.text`.

In `src/ui/chat.rs`, the placeholder `li` matches on the status instead of comparing it:

```rust
if let Status::Replying(text) = conversation.status() {
    li {
        class: "{Styles::chat_panel__turn}",
        "data-role": "assistant",
        if text.is_empty() { "..." } else { "{text}" }
    }
}
```

The Reset button's `disabled` already reads `!= Status::Idle`, so it needs no change.

The fake, inside `submit`'s spawned block, between `complete` and `settle`:

```rust
let outcome = gemini.complete(&history).await;
if let Ok(reply) = &outcome {
    for word in reply.text.split_inclusive(' ') {
        chat.write().append(word);
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}
chat.write().settle(outcome);
```

This needs `tokio = { version = "1", features = ["time"] }` under `[dependencies]` in
`Cargo.toml`. Dioxus desktop and mobile already run on a tokio runtime, so the sleep has
a timer to run on. You only have to name the crate and turn on its `time` feature. Step 4
deletes the loop and the dependency.

**Why it works.**

- **Why the text lives *inside* the variant.** With a separate `partial: String` field,
  `Idle` with leftover partial text would be a state the type allows, and every reader of
  `Conversation` would have to remember to ignore it. When `Replying` carries the text, that
  state can't be written at all. This is the Rust idiom of making illegal states
  unrepresentable. `if let Status::Replying(text) = &mut self.status` then *borrows into*
  the enum and edits the string where it is. Nothing is cloned or replaced.
- **Why `append` ignores a stray delta instead of panicking.** Step 2's *Stop* can land
  while the last delta is on its way. A delta that arrives after the reply ended is harmless
  and has nowhere to go, so dropping it is correct, not a workaround.
- **Why each `chat.write()` shows up on screen.** The component read `chat` during render,
  so it is subscribed. Each write marks it dirty, and Dioxus re-renders it on the next
  frame. The spawned task never touches the DOM. It only writes the signal, just as in
  Phase 19, only more often. Because the write guard is a temporary that is dropped at the
  `;`, it is released before the `.await`. A guard held across the sleep would make any
  render in the meantime panic on the borrow.
- **Why `split_inclusive(' ')`.** It keeps the space on the end of each word, so the pieces
  put together again equal the original, and `settle`'s `reply.text` matches what was on
  screen. Plain `split` would drop the spaces.

**Scope note.** Nothing streams yet: the pause before the first word is still the full
Gemini round-trip. *Stop* is Step 2. A failure partway through a reply can't happen with
this fake, because `complete` either succeeded before the loop or the loop never runs.
Step 4 adds that case.
