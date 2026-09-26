# Phase 22 — Streaming replies

[← Feature: Streaming replies](README.md) · **Status:** 🚧 in progress — opened 2026-09-25 ·
build log: [`phase-22-streaming-steps.md`](phase-22-streaming-steps.md)

## Goal

Ask a question in the chat drawer and the answer types itself in as it arrives. While it
is arriving, *Stop* ends the reply: the text so far stays in the conversation, and the next
question can be asked right away. The phase closes when a real Gemini reply streams into
the drawer on desktop and on the iOS simulator, and *Stop* cuts one off partway.

> **Taken ahead of Phase 21** on 2026-09-25, at the learner's call. Neither phase depends
> on the other. Phase 21 is FFI into UIKit's edit menu. This phase only touches the provider
> and the chat drawer. Phase 21 reuses Phase 20's hand-off either way.

## The crux

**A reply stops being one value and becomes a sequence over time.** That brings two new
ideas, one on each side of the provider boundary:

1. **In the network layer, the bytes don't arrive in event-sized pieces.** Gemini sends
   Server-Sent Events: lines of `data: {json}` separated by blank lines. TCP delivers
   *chunks*, and a chunk boundary can fall anywhere: halfway through a JSON object, or
   between the two bytes of an `é`. So a buffer must collect bytes and give back only
   *complete* lines. That buffer is a small, pure state machine you can test without the
   network, and it is the phase's one genuinely hard idea.
2. **In the conversation, the assistant turn grows while you watch.** `Status::Waiting`
   becomes `Status::Replying(String)`: the enum variant *carries* the partial text. The
   partial text then can't exist unless a reply is in progress, so no state can hold stray
   leftover text. Each delta is one `chat.write()`, which schedules a render. Nothing about
   Dioxus changes. The spawned task just writes a signal many times instead of once.

Cancelling is almost free. `Task::cancel` drops the future at whatever `.await` it is
parked on. Dropping the `reqwest::Response` closes the connection, so Gemini stops
generating and stops billing tokens.

## Design decisions (recorded up front)

- **The partial lives in the status, not in `messages`.** `messages` keeps holding
  *finished* turns only, which is exactly what the next request sends as history. A stopped
  reply becomes a finished turn when `stop()` moves it across.
- **The trait gets a callback, not a `Stream`.** Roughly
  `async fn stream(&self, messages: &[Message], on_text: impl FnMut(&str)) -> Result<Reply, ChatError>`.
  A `futures::Stream` is the more general shape, but it brings `Pin`, the `futures` crate and
  `StreamExt` in the same step. The callback keeps the trait to one idea. The full text is
  still returned at the end, so `settle` does not change.
- **`Response::chunk()`, not `bytes_stream()`.** `chunk().await` returns the next piece of
  the body with no extra `reqwest` feature and no `Stream` trait. It is a `while let` loop.
- **A stream failure keeps the question and drops the partial**, the same as a failed
  `complete` today. A *stopped* reply keeps its partial, because the user chose to stop it.
- **Plain text only.** Markdown rendering of replies is a separate idea and not in this
  phase.

## Planned steps

Detail for each lives in [`phase-22-streaming-steps.md`](phase-22-streaming-steps.md).

The order is **a visible growing reply first, faked from the real answer; then Stop against
that fake; then the parser; then the real stream swapped in**. Step 3 is the one exception
to observability order: the SSE buffer is the phase's hard idea, and a parser is best built
against tests before anything runs through it.

- [x] **1. The reply types itself in** — `Status::Waiting` → `Status::Replying(String)`,
      `Conversation::append`; the drawer renders the partial turn; the real `complete`
      answer is fed back word by word with a short sleep (faked at the edge); `#[test]` +
      `dx serve` eyeball.
- [x] **2. Stop** — `Conversation::stop` keeps the partial as an assistant turn; a *Stop*
      button while replying cancels the task; `#[test]` + `dx serve` eyeball.
- [x] **3. The SSE line buffer** — `src/ai/sse.rs`: bytes in, complete `data:` payloads out,
      correct across split lines, CRLF and a split multi-byte character; `#[test]` only.
- [ ] **4. The real stream** — `ChatProvider::stream` with an `on_text` callback; Gemini's
      `:streamGenerateContent?alt=sse` read with `chunk()` through the buffer; the fake
      loop and its `tokio` dependency removed; `#[test]` against the `Fake` + `dx serve` and
      iOS simulator eyeball.
- [ ] **5. Review and refactor** — whether `complete` still earns its place, the
      per-chunk text extraction shared with `reply_from`, names; suite green, clippy clean.
