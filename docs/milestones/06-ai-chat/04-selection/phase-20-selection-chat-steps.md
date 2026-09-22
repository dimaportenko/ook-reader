# Phase 20 — Selection → prefilled chat (desktop) — build log

[← Phase 20](phase-20-selection-chat.md) · the phase doc holds the goal, decisions and the
step index; this file holds the test → code → why for each step, newest at the bottom.

## The crux

The selection is in the chapter iframe, the button is in the host page, and the state it
must change belongs to a sibling component. Lift `open` and `draft` to `Reader`, the common
owner, and pass the signals down, since a `Signal` is a `Copy` handle to shared state. Get
the text across documents by *asking* on click: an `async` handler `eval`s a script that
returns the active frame's selection. Keep only a boolean flowing the other way, to show
and hide the button.

## Step plan

1. Ask AI opens a prefilled drawer — lifted signals, a hard-coded passage, real title + chapter.
2. A compose box that holds a quote — `<textarea>`, Enter sends, Shift+Enter breaks the line.
3. The real selection — `selectedText()` on the controller, an `async` handler that `eval`s it.
4. The author — `OpenBook.author` into the `Passage`.
5. Show *Ask AI* only while something is selected — `ook-selection` → `BridgeMsg::Selection`.
6. Review and refactor.

**Why this order.** Dependency order would start at the bottom: the selection bridge, then
the controller method, then the button. That means two steps whose only check is a string
test. Instead, Step 1 is the button the user will eventually press, wired to a fake passage,
so the whole path from click to drawer to draft is on screen immediately. Step 1 also shows
a real bug: the `<input>` flattens the quote onto one line. That is Step 2's reason to exist.
The real selection replaces the fake in Step 3 as a one-line swap inside a handler that
already works. The flag that hides the button comes last because it is polish, and it is
the only step that adds a new message to the bridge.

## Step 1 — Ask AI opens a prefilled drawer

**What it is.** A temporary text button, *Ask AI*, in the reader's control row next to
the chat icon. Pressing it writes `prompt::draft(..)` into the chat's compose box and opens
the drawer. The passage is a hard-coded sentence; the title and chapter label are real.
For this to work, the drawer's `open` and `draft` must be writable from outside
`ChatPanel`, so they move up into `Reader`.

**Check first: `dx serve`, eyeball.**

1. Open a book and page into a named chapter. Press *Ask AI*. The drawer slides in, and
   the compose box holds a draft that begins `I'm reading *<title>*, chapter "<label>".`
   followed by the hard-coded sentence.
2. The draft is **on one line**, with the `> ` quote run into the header. That is
   expected: `<input>` strips newlines. Step 2 fixes it. Note it and move on.
3. Turn a few pages into another chapter and press *Ask AI* again. The label in the draft
   is the **new** chapter.
4. Close the drawer with its ✕ and reopen it with the chat icon. It still opens and
   closes, and the draft is still there because nobody cleared it.
5. Send the draft with a real key. It lands as a user turn, and the compose box clears.
6. `cargo clippy --all-targets` is clean.

**Minimal implementation.**

`src/ui/chat.rs`: `ChatPanel` stops creating `open` and `draft` and takes them as props
instead. Delete the two `use_signal` lines, and nothing else in the body changes:

```rust
#[component]
pub(crate) fn ChatPanel(
    show_controls: bool,
    mut open: Signal<bool>,
    mut draft: Signal<String>,
) -> Element {
    let provider = use_context::<Signal<Option<Gemini>>>();
    let mut chat = use_signal(Conversation::default);
    // …the rest is unchanged: `submit`, the button, the aside…
```

`src/ui/reader.rs`: `Reader` creates the two signals, and a button fills them. Near the
other `use_signal` calls:

```rust
let mut chat_open = use_signal(|| false);
let mut chat_draft = use_signal(String::new);
```

In the right-hand control `div`, before `ChatPanel`, add the button, and pass the signals to
`ChatPanel`:

```rust
button {
    onclick: {
        let title = book.title.clone();
        let chapter = chapter_label.clone();
        move |_| {
            chat_draft.set(prompt::draft(&Passage {
                title: &title,
                author: None,
                chapter: Some(&chapter),
                text: "Holmes sat silent for a few minutes with his fingertips still pressed together.",
            }));
            chat_open.set(true);
        }
    },
    "Ask AI"
}
ChatPanel {
    show_controls: show_controls(),
    open: chat_open,
    draft: chat_draft,
}
```

Plus `ai::prompt::{self, Passage}` in the `use crate::{…}` block.

**Why it works.**

- **Lifting state is how siblings share it.** Dioxus data flows down through props, and
  a component cannot reach into a sibling's hooks. `Reader` is the lowest component that
  renders both the button and the drawer, so the state moves there. `Signal<T>` is `Copy`:
  `open: chat_open` hands `ChatPanel` a second handle to the *same* slot, not a copy of the
  `bool`. `ChatPanel` reads `open()` during render and so subscribes to it. When `Reader`'s
  button writes it, `ChatPanel` re-renders. Ownership of the value stays with the
  `Reader` scope, so the draft now lives as long as the reader does, just as the
  conversation does.
- **Why `mut` in the component's signature.** `.set()` takes `&mut self`, so the binding
  must be mutable. It costs nothing because the handle is `Copy`. It is the same reason
  `let mut chat_open` needs `mut` in `Reader`.
- **Why the `{ let …; move |_| … }` block around the handler.** The closure is `move`,
  so it owns what it captures. `book.title` and `chapter_label` are `String`s that the rest
  of the render still uses, so they cannot be moved into the closure. The block clones them
  first and moves the clones. `Passage` borrows `&title` and `&chapter` from those clones
  only for the length of the `draft` call.
- **Why the chapter label is right after paging (check 3).** A component re-runs
  top to bottom on every render, and each run builds a *new* `onclick` closure.
  `chapter_label` is recomputed from the `chapter()` signal on each render, so each
  closure captures the label that was on screen when it was built. A signal is different:
  it is read when the handler runs, not when it is built. Step 3 relies on that
  difference when the selection has to be read at click time.

**Scope note.** The passage is fake until Step 3; `author` is `None` until Step 4; the
button is always visible until Step 5, and it is hidden along with the other controls by a tap,
which Step 5 changes too. The one-line draft is Step 2. Focusing the compose box when the
drawer opens with a draft is not planned; if it itches after Step 2, raise it in Step 6.
