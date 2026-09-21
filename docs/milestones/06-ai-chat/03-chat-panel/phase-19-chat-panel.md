# Phase 19 — Chat panel

[← Feature: Chat panel](README.md) · **Status:** 🚧 in progress — opened 2026-09-20 ·
build log: [`phase-19-chat-panel-steps.md`](phase-19-chat-panel-steps.md)

## Goal

A chat button in the reader's control row opens a drawer beside the page. The drawer lists
the conversation so far, takes a question in an input, sends it through the provider in
context, and shows the answer when it arrives — or the error when it does not. With no key
set it says so and points at settings. The phase closes when a question typed on desktop
gets a real Gemini answer back into the list, and the same drawer opens on the iOS
simulator.

## The crux

**The network call is the first `.await` in the UI, and a component cannot wait.** A Dioxus
component function runs to completion on every render; nothing in it may block. The shape
that works is *fire and forget with a handle*: an event handler calls `spawn` with an async
block, the block `.await`s the provider, and when the reply lands it **writes a signal**,
which schedules the render that shows it. The component never waits — it renders the
"waiting" state now and the "answered" state later, both from the same signal.

Two consequences shape the steps. First, **a signal's read guard must not cross an
`.await`**: `provider.read()` borrows the signal, and holding that borrow while suspended
means any write in between panics. Clone the `Gemini` out (it is a small `Clone` value)
before the async block, and read nothing else inside it. Second, **the state machine is not
the component.** *Asked → waiting → answered | failed* is three transitions on a `Vec` and
an enum; put them on a plain `Conversation` struct so they are `#[test]`-able against the
`Fake` provider, and let the component be a thin shell that owns one `Signal<Conversation>`
and calls those methods.

## Design decisions (recorded up front)

- **A drawer, not a route.** The chat opens over the reader like the contents popover does,
  because Phase 20 prefills it from a selection and the reader must stay visible behind it.
  Reversible: a route is a `Link` and a component swap.
- **A new module, `src/chat/`**, owning `Conversation` and its `Status`. It depends on `ai`
  (for `Message`, `Reply`, `ChatError`) and nothing else; `ui/chat.rs` depends on it.
- **`Message` grows read accessors** (`role()`, `text()`) in `ai/mod.rs`. Its fields stay
  private; the UI reads, it never constructs a `Message` by hand.
- **One conversation per open book**, held in a signal created in `Reader`, so closing the
  book drops it. Persisting chats is not a Phase 19 requirement.
- **The whole history goes on every request.** Gemini's `generateContent` is stateless; the
  request body already accepts the full `contents` list. Trimming is Phase 22's problem.
- **A failed send keeps the question.** `Status::Failed(text)` sits beside the messages; the
  user turn stays in the list so re-asking is one click, not a retype.

## Planned steps

Detail for each lives in [`phase-19-chat-panel-steps.md`](phase-19-chat-panel-steps.md).

The order is **UI first, then the state behind it, then the network** — each step leaves
something to click, and the pure-Rust struct arrives exactly when the drawer has a submit
that needs it. (Re-ordered 2026-09-20 from a state-first plan; see the build log's "Why this
order".)

- [x] **1. The drawer shell** — a chat button in the reader's control row toggles a
      `ChatPanel` drawer; with `None` in the provider signal it reads "add a key in
      settings"; `dx serve` eyeball, `cargo clippy`, then the iOS simulator for drawer
      geometry against the safe area.
- [x] **2. The message list and input** — the drawer renders a `Signal<Vec<Message>>`
      seeded with one user and one assistant turn, so both roles are visible; submit
      appends a trimmed user turn and clears the input; no network; `dx serve` eyeball.
- [ ] **3. The conversation state** — `src/chat/mod.rs`: `Conversation` with `ask` and
      `settle`, `Status`; `Message::role`/`text` accessors; `#[test]` with `cargo test`.
      The drawer moves onto `Signal<Conversation>`, submit calls `ask`, the seed goes.
- [ ] **4. The async send** — submit `spawn`s `provider.complete` on the full history and
      `settle`s the result; a waiting row and an error row; `dx serve` with a real key on
      desktop, then the iOS simulator.
- [ ] **5. Review and refactor** — punch-list over `src/chat/`, `ui/chat.rs`, and the
      accessor change in `ai`; suite green, clippy clean.
