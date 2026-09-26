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

1. ~~The reply types itself in~~ — `Replying(String)`, `append`, the real answer fed back word by word. **Done** — `a158b50`.
2. ~~Stop~~ — `Conversation::stop`, a *Stop* button that cancels the task. **Done** — `8aecc9a`.
3. ~~The SSE line buffer~~ — `src/ai/sse.rs`, pure, under `#[test]`. **Done** — `8f1bfa9`.
4. ~~The real stream~~ — `ChatProvider::stream` with `on_text`, Gemini `alt=sse`, the fake removed. **Done** — `6f03994`.
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

> **Status:** done — committed in `a158b50` (193 tests green, 2 ignored; clippy clean; `dx serve` checks confirmed by eye).

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

## Step 2 — Stop

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `8aecc9a` (196 tests green, 2 ignored; clippy clean; `dx serve` checks confirmed by eye).

**What it is.** While a reply is typing itself in, the drawer's *Send* button becomes
*Stop*. Pressing it ends the reply where it is. The words already on screen stay as an
assistant turn, and the next question can be asked straight away. The Step 1 fake's 40 ms
sleep between words is what makes a reply slow enough to stop.

**Check first — `cargo test chat::`.** Add these to the `test` module in
`src/chat/mod.rs`:

```rust
#[test]
fn stopping_keeps_the_partial_as_an_assistant_turn() {
    let mut chat = Conversation::default();
    chat.ask("Which city?");
    chat.append("Ankh-");

    chat.stop();

    assert_eq!(
        chat.messages(),
        &[Message::user("Which city?"), Message::assistant("Ankh-")]
    );
    assert_eq!(chat.status(), &Status::Idle);
    assert!(chat.ask("And the river?"), "a stopped chat accepts a new turn");
}

#[test]
fn stopping_before_the_first_word_keeps_only_the_question() {
    let mut chat = Conversation::default();
    chat.ask("Which city?");

    chat.stop();

    assert_eq!(chat.messages(), &[Message::user("Which city?")]);
    assert_eq!(chat.status(), &Status::Idle);
}

#[test]
fn stopping_with_no_reply_in_progress_changes_nothing() {
    let mut chat = Conversation::default();
    chat.ask("Which city?");
    chat.settle(Err(ChatError::Empty));

    chat.stop();

    assert_eq!(chat.messages(), &[Message::user("Which city?")]);
    assert_eq!(
        chat.status(),
        &Status::Failed("the provider returned no answer".to_owned())
    );
}
```

They fail to compile at first (`no method named stop`). That counts as red.

**Then the eyeball — `dx serve`.** Open the chat and ask for something long:

1. While the "..." bubble shows and while words are appearing, the right-hand button reads
   *Stop*, not *Send*.
2. Press *Stop* partway through. The words stop coming, the bubble keeps what it had and
   looks like an ordinary assistant turn, and the button goes back to *Send*.
3. Ask a follow-up. It sends, and the reply refers to the cut-off answer, because the
   partial is part of the history now.
4. Press *Stop* during the "..." (before any words). The bubble disappears, the question
   stays, and *Send* is back.

**Minimal code.**

In `src/chat/mod.rs`:

```rust
pub(crate) fn stop(&mut self) {
    let Status::Replying(text) = &mut self.status else {
        return;
    };
    let text = std::mem::take(text);
    self.status = Status::Idle;

    if !text.is_empty() {
        self.messages.push(Message::assistant(text));
    }
}
```

In `src/ui/chat.rs`, a `stop` closure takes over `reset`'s cancel lines, and `reset`
becomes "stop, then start over":

```rust
let mut stop = move || {
    if let Some(task) = pending_task.take() {
        task.cancel();
    }
    chat.write().stop();
};

let mut reset = move || {
    stop();
    chat.set(Conversation::default());
};
```

and the *Send* button becomes one branch of an `if`:

```rust
if matches!(conversation.status(), Status::Replying(_)) {
    button {
        class: "{Styles::chat_panel__compose_action}",
        onclick: move |_| stop(),
        "Stop"
    }
} else {
    button { /* Send, unchanged */ }
}
```

**Why it works.**

- **`Task::cancel` is the whole of stopping the work.** The spawned future is parked at
  one of its `.await`s: `complete`, or the 40 ms sleep. Cancelling drops the future right
  there, so the code after that `.await` never runs. No more `append`, no `settle`, no
  `pending_task.set(None)`. That is why the closure `take()`s the task itself. In Step 4 the
  future owns the open `reqwest::Response`, so dropping it closes the connection too.
- **`let … else` then `mem::take`.** `let Status::Replying(text) = &mut self.status else {
  return; }` is the early return from `ask`, written as a pattern: any other status is left
  exactly as it was, including `Failed`. `text` is a `&mut String` pointing into the enum.
  `std::mem::take` moves the `String` out and leaves an empty one behind, so we own the
  text without cloning it. After that line nothing uses the borrow, so assigning
  `self.status = Status::Idle` compiles. Assigning first would not: the borrow would still
  be needed for the `take`.
- **Why an empty partial adds no turn.** An empty assistant message would be a blank bubble
  in the drawer, and a blank turn in the history sent to Gemini. The question stays on its
  own, the same as after a failed `complete`. Phase 19 already allows two user turns in a
  row that way.
- **Why `reset` can call `stop` and `stop` can still be used later.** A closure is `Copy`
  when everything it captures is `Copy`. `stop` captures only two `Signal`s, which are
  `Copy` handles, so `move` into `reset` copies `stop` instead of moving it away, and the
  *Stop* button's `move |_| stop()` gets its own copy too. `reset` briefly writes the
  partial turn into the conversation and then replaces the whole conversation. Both writes
  happen in the same event handler, so Dioxus renders once.
- **Why Send turns into Stop, not a third button.** `ask` refuses while a reply is in
  progress, so *Send* has nothing to do during a reply. Showing one button whose job
  matches the state is also the familiar chat-app pattern.

**Scope note.** The fake is still in place, so stopping during "..." cancels `complete`
before Gemini has answered. The request is dropped, but the client can't tell Gemini to
stop generating until Step 4's stream owns the connection. The
Escape key and a keyboard shortcut for *Stop* are out of scope.

## Step 3 — The SSE line buffer

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `8f1bfa9` (202 tests green, 2 ignored; clippy clean).

**What it is.** A small pure type, `SseBuffer`, in `src/ai/sse.rs`. You feed it the body
of an HTTP response in whatever pieces the network hands over. It gives back the payload
of every complete `data:` line it has seen so far and keeps the unfinished tail for next
time. Nothing calls it yet. Step 4 feeds it `Response::chunk()`. This step is the exception
to observability order that the phase doc planned: the only check is `#[test]`.

**The crux, again.** Gemini's stream is text, `data: {json}\n\n` per event, but it arrives
as TCP chunks, and a chunk boundary can fall anywhere: in the middle of the JSON, between
`\r` and `\n`, or between the two bytes of an `é`. So the buffer must decide **nothing**
until it holds a whole line. One fact makes that safe: in UTF-8 the byte `0x0A` (`\n`)
never appears inside a multi-byte character. Every byte of a multi-byte sequence has its
high bit set. So cutting the *bytes* at `\n` never cuts a character, and decoding each
whole line as text can't fail on a split `é`.

**Check first — `cargo test ai::sse`.** In a new `src/ai/sse.rs`:

```rust
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn a_whole_event_yields_its_payload() {
        let mut sse = SseBuffer::default();

        assert_eq!(sse.push(b"data: {\"a\":1}\n\n"), ["{\"a\":1}"]);
    }

    #[test]
    fn a_line_split_across_chunks_waits_for_the_rest() {
        let mut sse = SseBuffer::default();

        assert!(
            sse.push(b"data: {\"a\"").is_empty(),
            "no newline yet, no line"
        );
        assert_eq!(sse.push(b":1}\n\n"), ["{\"a\":1}"]);
    }

    #[test]
    fn two_events_in_one_chunk_yield_both() {
        let mut sse = SseBuffer::default();

        assert_eq!(sse.push(b"data: one\n\ndata: two\n\n"), ["one", "two"]);
    }

    #[test]
    fn crlf_line_endings_are_stripped() {
        let mut sse = SseBuffer::default();

        assert_eq!(sse.push(b"data: one\r\n\r\n"), ["one"]);
    }

    #[test]
    fn a_character_split_across_chunks_survives() {
        let mut sse = SseBuffer::default();

        assert!(sse.push(b"data: caf\xC3").is_empty());
        assert_eq!(
            sse.push(b"\xA9\n\n"),
            ["caf\u{e9}"],
            "the two bytes of \u{e9} arrived in different chunks"
        );
    }

    #[test]
    fn lines_that_are_not_data_are_skipped() {
        let mut sse = SseBuffer::default();

        assert_eq!(
            sse.push(b": keep-alive\nevent: message\n\ndata: one\n\n"),
            ["one"]
        );
    }
}
```

and register it in `src/ai/mod.rs` as `#[cfg(test)] mod sse;`. To see real red rather than
a compile error, start with a stub `push` that returns `Vec::new()`: all six fail with
`left: []`.

**Minimal code.**

```rust
#[derive(Debug, Default)]
pub(crate) struct SseBuffer {
    pending: Vec<u8>,
}

impl SseBuffer {
    pub(crate) fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.pending.extend_from_slice(chunk);

        let mut payloads = Vec::new();
        while let Some(end) = self.pending.iter().position(|&byte| byte == b'\n') {
            let bytes: Vec<u8> = self.pending.drain(..=end).collect();
            let text = String::from_utf8_lossy(&bytes);
            let line = text.trim_end_matches(['\n', '\r']);

            if let Some(data) = line.strip_prefix("data:") {
                payloads.push(data.strip_prefix(' ').unwrap_or(data).to_owned());
            }
        }

        payloads
    }
}
```

**Why it works.**

- **The buffer holds bytes, not a `String`.** A `String` must always be valid UTF-8, and
  half an `é` isn't. Keeping `Vec<u8>` means the tail can end anywhere, and text only
  appears once a line is whole.
- **`position` then `drain(..=end)`.** `position` finds the first `\n`. `drain(..=end)`
  removes the line *including* its `\n` from the front of `pending` and hands those bytes
  over. Whatever follows (the next line, or half of one) slides to the front and waits.
  The `while let` repeats until no `\n` is left, which is how one chunk can yield two
  events.
- **Three names for three types.** `bytes` is the owned `Vec<u8>`, `text` the decoded
  `Cow<str>`, and `line` a `&str` borrowed from `text` without its line ending. Shadowing
  one name through all three would compile, but the type change is the point of those lines.
- **`from_utf8_lossy` returns a `Cow<str>`.** When the bytes are valid, which the `\n` fact
  guarantees for any line the server sent correctly, it borrows them with no copy. Only a
  truly broken line gets `U+FFFD` replacement characters. `Cow` derefs to `&str`, so
  `trim_end_matches` and `strip_prefix` work on it directly.
- **`trim_end_matches(['\n', '\r'])`** accepts an array of `char`s as the pattern and
  strips both ends of a CRLF line in one call.
- **`strip_prefix("data:")` returns `Option<&str>`.** That one call both tests the prefix and
  gives back the rest, so the `if let` is the whole filter. Blank lines (the `\n\n`
  between events), comments (`: keep-alive`) and `event:` lines all return `None`, so they
  drop out. The SSE spec removes exactly *one* optional space after the colon, which is
  what `strip_prefix(' ').unwrap_or(data)` does. `trim_start` would also eat spaces that
  belong to the payload.
- **Why `#[cfg(test)] mod sse;`.** Until Step 4 calls it, a normal build would warn that
  `SseBuffer` is never constructed. Gating the module to test builds keeps `cargo clippy`
  clean and makes the gate itself the reminder: Step 4 deletes `#[cfg(test)]` when the
  first caller arrives.

**Scope note.** The buffer yields one payload per `data:` line. The SSE spec joins several
`data:` lines of one event with `\n`, but Gemini sends one line per event, so joining them
would be code for a case this app never sees. `id:`, `retry:` and reconnecting are out of
scope for the same reason. There is no `flush()` for a last line that arrives without its
`\n` (SSE ends every event with a blank line, so a well-formed stream leaves nothing
behind), and no cap on how long `pending` may grow. Parsing the JSON payload into text is
Step 4.

## Step 4 — The real stream

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `6f03994` (206 tests green, 3 ignored; clippy clean; `dx serve` and iOS simulator checks confirmed by eye).

**What it is.** The fake goes away. `ChatProvider` gains `stream`, which takes an `on_text`
callback. Gemini's `stream` calls `:streamGenerateContent?alt=sse`, reads the body with
`Response::chunk()`, runs each chunk through Step 3's `SseBuffer`, parses each `data:`
payload as a `GenerateResponse`, and hands its text to `on_text`. The drawer's spawned task
now makes one call: `gemini.stream(&history, |delta| chat.write().append(delta))`. The
`tokio` `time` dependency goes too.

**Check first — `cargo test ai::`.** In `src/ai/mod.rs`, the `Fake` has to speak the new
method, and a test pins what the contract promises: the pieces arrive in order and add up to
the returned reply.

```rust
#[test]
fn a_provider_streams_its_answer_in_pieces() {
    let fake = Fake {
        reply: "Ankh-Morpork on the Ankh",
        seen: RefCell::new(Vec::new()),
    };
    let question = Message::user("Which city?");
    let mut pieces = Vec::new();

    let reply = pollster::block_on(fake.stream(std::slice::from_ref(&question), |delta| {
        pieces.push(delta.to_owned())
    }))
    .unwrap();

    assert_eq!(pieces, ["Ankh-Morpork ", "on ", "the ", "Ankh"]);
    assert_eq!(pieces.concat(), reply.text, "the pieces add up to the reply");
    assert_eq!(fake.seen.borrow().as_slice(), &[question]);
}
```

Red: `error[E0599]: no method named 'stream' found for struct 'Fake'`.

In `src/ai/gemini.rs`, three pure tests for the Gemini side. A streamed chunk has the same
JSON shape as a whole response, so a payload's text comes from the same parse. The closing
chunk carries `finishReason` and usage and an empty text. The endpoint asks for SSE:

```rust
#[test]
fn a_streamed_chunk_yields_its_text() { /* {"candidates":[{"content":{"parts":[{"text":"Ankh-"}]}}]} → "Ankh-" */ }

#[test]
fn a_closing_chunk_with_no_text_yields_nothing() { /* text "" + finishReason + usageMetadata → "" */ }

#[test]
fn the_stream_endpoint_asks_for_server_sent_events() { /* …:streamGenerateContent?alt=sse */ }
```

plus `a_real_gemini_streams_through_the_trait`, `#[ignore]`d like its `complete` twin, which
asks for "one to twenty in words" and checks that more than one piece arrived and that the
pieces add up to the reply.

**Then the eyeball — `dx serve`, then the iOS simulator.** Ask for something long:

1. The first words appear sooner than before. They no longer wait for the whole answer.
2. The text arrives in bursts of a few words (Gemini's chunk size), not one word at a time
   every 40 ms.
3. *Stop* partway keeps the partial, as in Step 2. Now it also closes the connection.
4. With a bad key, the error shows as before (`the provider rejected the request (400)…`).

**Minimal code.**

The trait, in `src/ai/mod.rs`:

```rust
async fn stream(
    &self,
    messages: &[Message],
    on_text: impl FnMut(&str),
) -> Result<Reply, ChatError>;
```

The `Fake` calls `on_text` once per `split_inclusive(' ')` word, then returns
`self.complete(messages).await`.

In `src/ai/gemini.rs`, the first-candidate text extraction moves out of `reply_from` into
`text_of(GenerateResponse) -> String`, and the empty check into
`non_empty_reply(String) -> Result<Reply, ChatError>`. `reply_from` becomes those two in a
row. Then:

```rust
async fn stream(&self, messages: &[Message], mut on_text: impl FnMut(&str)) -> Result<Reply, ChatError> {
    let mut response = self.client.post(stream_endpoint(&self.model))
        .header("x-goog-api-key", &self.key)
        .json(&request_body(messages))
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        return Err(ChatError::Api { status: status.as_u16(), body: response.text().await? });
    }

    let mut sse = SseBuffer::default();
    let mut text = String::new();
    while let Some(chunk) = response.chunk().await? {
        for payload in sse.push(&chunk) {
            let delta = text_of(serde_json::from_str(&payload)?);
            on_text(&delta);
            text.push_str(&delta);
        }
    }

    non_empty_reply(text)
}
```

`mod sse;` loses its `#[cfg(test)]`, because it has a caller now. In the other direction,
`complete` (on the trait and on `Gemini`), `endpoint`, `check_status`, `reply_from` and the
`StatusCode` import *gain* `#[cfg(test)]`, because `stream` took their last caller. Step 5
decides whether they go.

In `src/ui/chat.rs`, the fake loop and `use std::time::Duration` go:

```rust
let outcome = gemini
    .stream(&history, |delta| chat.write().append(delta))
    .await;
chat.write().settle(outcome);
```

`tokio = { version = "1", features = ["time"] }` leaves `[dependencies]`. The dev-dependency
stays for `#[tokio::test]`.

**Why it works.**

- **`chunk()` + `while let`.** `Response::chunk()` returns `Result<Option<Bytes>>`: `?`
  takes care of the network error, and `Some` / `None` means another piece / end of body.
  That is a stream read with no `Stream` trait. `&chunk` derefs from `Bytes` to `&[u8]`,
  which is exactly what `SseBuffer::push` takes.
- **The status is checked before the body is read.** A failed request answers with one JSON
  error document, not an event stream. So on a non-2xx the whole body is read as text for
  the error, the same as `complete`. `check_status` can't be reused here, because it takes
  the body *first*, and on success that would read the whole stream before a single word
  showed.
- **Why the closure can write the signal.** `chat` is a `Signal`, a `Copy` handle, moved into
  the `async move` block. The closure borrows it mutably for as long as the `stream` future
  runs. That borrow ends at `.await`'s completion, so `chat.write().settle(outcome)` on the
  next line compiles. Each `write()` guard is dropped at the end of the closure call, so no
  guard lives across an `.await` inside `stream` (clippy's `await_holding_invalid_type` would
  catch that).
- **Why the full text is still returned.** `settle` pushes `reply.text` as the finished
  turn, which is the same text `append` built up in `Status::Replying`. Keeping the return
  value left `settle` unchanged, as the phase doc decided. The cost is that the text is
  accumulated twice, once in `stream` and once in the conversation.
- **Errors partway through.** A dropped connection (`chunk()` fails) or a malformed payload
  (`from_str` fails) returns `Err` through `?`. `settle` turns that into `Status::Failed`, and
  the partial is dropped, as decided up front. A reply whose every chunk carried no text
  (a blocked prompt) is `ChatError::Empty` through `non_empty_reply`, as before.
- **Cancel closes the connection now.** Stop's `Task::cancel` drops the future parked on
  `chunk().await`. That future owns `response`, so dropping it closes the connection, and
  Gemini stops generating.

**Scope note.** Gemini sometimes sends an empty `text` on the final chunk, and `on_text("")`
is harmless (`append` of nothing), so it isn't filtered out. Whether `complete` stays, the
duplicated base URL between `endpoint` and `stream_endpoint`, and the two copies of the
`Api` error are all Step 5. A candidate with no `content` at all (a mid-stream safety stop)
would fail to parse as `Json`, not `Empty`. `complete` has the same gap, and it is also a
Step 5 candidate.
