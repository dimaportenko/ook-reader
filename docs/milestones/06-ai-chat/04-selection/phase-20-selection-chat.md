# Phase 20 — Selection → prefilled chat (desktop)

[← Feature: Selection → prefilled chat](README.md) · **Status:** 🚧 in progress — opened 2026-09-22 ·
build log: [`phase-20-selection-chat-steps.md`](phase-20-selection-chat-steps.md)

## Goal

Select a passage in a chapter on desktop and an *Ask AI* button appears. Press it: the chat
drawer opens with the compose box holding `prompt::draft` of the book's title and author,
the current chapter label and the selected text, quoted line by line. The cursor waits
below the quote for the question. The phase closes when a real selection on desktop becomes a
draft, the question is typed under it, and Gemini answers it.

## The crux

**The selection lives in a different document from the button.** The passage is selected
inside a sandboxed chapter `<iframe>`; *Ask AI* is a Dioxus button in the host page; and the
state that has to change (the drawer's `open` and `draft`) is owned by `ChatPanel`, a
sibling of that button. So there are two hand-offs to make, and each has one idea behind it:

1. **Across components: lift the state to the common owner.** `Reader` owns both the
   button and `ChatPanel`, so it creates the `open` and `draft` signals and passes them
   down as props. A `Signal` is a `Copy` handle, so passing it does not copy the value. Both
   components hold the same signal, and a write in one re-renders the other.
2. **Across documents: ask when you need it.** Rust cannot read the DOM. It can run a
   script with `document::eval` and `.await` the value the script returns. The click
   handler becomes `async`: ask the reader controller for the active frame's
   `getSelection().toString()`, wait for the answer, then build the draft. Nothing about the
   selection is stored between clicks. The only thing pushed continuously is a boolean,
   *is anything selected?*, and that comes last, just to show and hide the button.

## Design decisions (recorded up front)

- **`Reader` owns `open` and `draft`; `ChatPanel` keeps the `Conversation`.** The conversation's
  lifetime does not change (it still dies with the open book). Only the two values that
  something outside the drawer must write are lifted.
- **The compose box becomes a `<textarea>`.** `prompt::draft` is multi-line by design
  (a header, a blank line, `> `-quoted lines, a trailing blank line). An `<input>` strips
  newlines from its value. **Enter sends and Shift+Enter inserts a newline**, the usual
  chat convention.
- **Pull the selection text; push only a flag.** The text is read on click through a new
  `selectedText()` method on the reader controller, next to `resolveGesture`. The
  frame reports only a `selection:true|false` bridge message, and only when the value flips.
  Streaming the text on every `selectionchange` would put a round-trip on every drag.
- **The draft replaces the compose box.** Pressing *Ask AI* sets `draft` outright, even if
  something half-typed was there. The conversation is untouched: *Ask AI* on a second passage
  continues the same chat.
- **An empty selection still opens the chat**, leaving the draft as it was. The button is
  hidden in that state from Step 5 on, so this only covers the race where the selection
  clears between the flag and the click.
- **`OpenBook` gains `author`.** The `books` row already has it; the reader never needed it
  until now.

## Planned steps

Detail for each lives in [`phase-20-selection-chat-steps.md`](phase-20-selection-chat-steps.md).

The order is **entry point first with a faked passage, then the compose box that can show
it, then the real selection, then the remaining metadata, then the show/hide polish**. Each
step ends with something to click. The only real abstraction (`prompt::draft`) was already
built and tested in Phase 17.

- [x] **1. Ask AI opens a prefilled drawer** — lift `open` and `draft` from `ChatPanel`
      into `Reader`; a temporary *Ask AI* button in the control row fills the draft from a
      hard-coded passage plus the real title and chapter label, then opens the drawer;
      `dx serve` eyeball, `cargo clippy`.
- [ ] **2. A compose box that holds a quote** — `<textarea>` in place of `<input>`; Enter
      sends, Shift+Enter breaks the line, an IME's confirming Enter does not send — the rule
      in a pure `sends` fn under `#[test]`; `dx serve` eyeball.
- [ ] **3. The real selection** — `selectedText()` on the reader controller; the *Ask AI*
      handler goes `async`, `eval`s it and drafts from the answer; an empty selection only
      opens the drawer; a three-hop agreement `#[test]` + `dx serve` eyeball.
- [ ] **4. The author** — `OpenBook.author` from the `books` row, passed into the
      `Passage`; `dx serve` eyeball on a book with and one without an author.
- [ ] **5. Show *Ask AI* only while something is selected** — a `selectionchange`
      listener in the chapter posts `ook-selection` when the flag flips, the controller
      forwards it as `selection:true|false` (and `false` on a chapter change),
      `BridgeMsg::Selection(bool)` drives a `Signal<bool>`; parse + three-hop `#[test]`,
      `dx serve` eyeball.
- [ ] **6. Review and refactor** — punch-list over the lifted props, the async handler, the
      new bridge message and the controller method; whether `#[allow(dead_code)]` on
      `mod ai` can go now that `prompt` has a caller; suite green, clippy clean.
