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

1. ~~Ask AI opens a prefilled drawer~~ — lifted signals, a hard-coded passage, real title + chapter. **Done** — `742e2f7`.
2. ~~A compose box that holds a quote~~ — `<textarea>`, Enter sends, Shift+Enter breaks the line. **Done** — `b499cf3`.
3. ~~The author~~ — `OpenBook.author` into the `Passage`. **Done** — `21b4f65`.
4. ~~The real selection~~ — `selectedText()` on the controller, an `async` handler that `eval`s it. **Done** — `f30cfde`.
4½. ~~Send on open~~ *(manual)* — `autosend` flag, toolbar chat icon, Reset. **Done** — `4218631`.
5. Show *Ask AI* only while something is selected — `ook-selection` → `BridgeMsg::Selection`.
6. Review and refactor.

**Why this order.** Dependency order would start at the bottom: the selection bridge, then
the controller method, then the button. That means two steps whose only check is a string
test. Instead, Step 1 is the button the user will eventually press, wired to a fake passage,
so the whole path from click to drawer to draft is on screen immediately. Step 1 also shows
a real bug: the `<input>` flattens the quote onto one line. That is Step 2's reason to exist.
The real selection replaces the fake in Step 4 as a swap inside a handler that already
works. (Steps 3 and 4 were swapped on 2026-09-22 because the author landed first in the
working tree, and neither step depends on the other.) The flag that hides the button comes last because it is polish, and it is
the only step that adds a new message to the bridge.

## Step 1 — Ask AI opens a prefilled drawer

> **Status:** done — committed in `742e2f7` (175 tests green, 2 ignored; clippy clean; `dx serve` checks confirmed by eye).

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
  it is read when the handler runs, not when it is built. Step 4 relies on that
  difference when the selection has to be read at click time.

**Scope note.** The passage is fake until Step 4; `author` is `None` until Step 3; the
button is always visible until Step 5, and it is hidden along with the other controls by a tap,
which Step 5 changes too. The one-line draft is Step 2. Focusing the compose box when the
drawer opens with a draft is not planned; if it itches after Step 2, raise it in Step 6.

## Step 2 — A compose box that holds a quote

> **Status:** done — committed in `b499cf3` (178 tests green, 2 ignored; clippy clean; `dx serve` checks confirmed by eye).

> **Written by:** `lbb:next-implement`. The agent wrote the implementation and tests;
> they are reviewed by hand.

**What it is.** The compose `<input>` becomes a `<textarea>`, so the draft from Step 1 shows
as the lines `prompt::draft` wrote: a header, a blank line, `> `-quoted lines, and an
empty line for the question. Enter still sends. Shift+Enter now inserts a newline, and
Enter pressed while an input method is composing (Japanese, Chinese, dead-key accents)
confirms the composition instead of sending. That three-way rule lives in a small
pure function so it can be tested.

**Check first (part 1): `cargo test ui::chat`.** Add this to the existing `mod test` in
`src/ui/chat.rs`. It fails to compile until `sends` exists:

```rust
use dioxus::prelude::Key;

use super::sends;

#[test]
fn enter_sends_but_shift_enter_breaks_the_line() {
    assert!(sends(&Key::Enter, false, false));
    assert!(!sends(&Key::Enter, true, false), "Shift+Enter is a newline");
}

#[test]
fn enter_that_confirms_an_ime_composition_does_not_send() {
    assert!(
        !sends(&Key::Enter, false, true),
        "the Enter that picks a kana candidate would otherwise send half a word",
    );
}

#[test]
fn only_enter_sends() {
    assert!(!sends(&Key::Character("a".into()), false, false));
    assert!(!sends(&Key::Tab, false, false));
}
```

**Check first (part 2): `dx serve`, eyeball.**

1. Press *Ask AI*. The compose box shows the draft **on several lines**: the
   `I'm reading …` header, a blank line, the `> ` quote, then an empty line where the
   cursor goes. It is tall enough to show the quote without scrolling.
2. Click at the end, type a question, and press Enter. The turn lands in the list with its
   line breaks intact, as the quote and then the question. This needs the one-line CSS
   change to `.chat_panel__turn` below; without it, the turn is flattened.
3. Type `one`, press Shift+Enter, type `two`. The box has two lines and nothing was sent.
4. An empty box with just Shift+Enter newlines still leaves Send disabled, and Enter
   does not send, because `ask` trims and refuses a blank turn.
5. `cargo clippy --all-targets` is clean.

**Minimal implementation.**

`src/ui/chat.rs`, a free function below the component:

```rust
fn sends(key: &Key, shift: bool, composing: bool) -> bool {
    *key == Key::Enter && !shift && !composing
}
```

In the compose `div`, replace `input { … }` with a `textarea`. Only the element name, one
attribute and the key test change:

```rust
textarea {
    rows: 6,
    value: "{draft}",
    placeholder: "Ask about this book",
    oninput: move |e| draft.set(e.data.value()),
    onkeydown: move |e| {
        if sends(&e.key(), e.modifiers().shift(), e.is_composing()) {
            e.prevent_default();
            submit();
        }
    },
}
```

`src/ui/chat.css`: the `input` selector becomes `textarea`, and the box gets the page's
font and a vertical-only resize handle:

```css
.chat_panel__compose textarea {
  flex: 1;
  font: inherit;
  resize: vertical;
}
```

and `.chat_panel__turn` gains `white-space: pre-wrap;`.

**Why it works.**

- **Why the `<input>` lost the newlines.** The HTML spec's *value sanitization* for a
  single-line text field strips every CR and LF from the value. The string in `draft` was
  always correct. The browser discarded the line breaks when it wrote the string into the
  element. A `<textarea>` is the multi-line control and keeps them. `value:` on a
  `textarea` in Dioxus sets the DOM **property**, so it stays a controlled field like the
  `input` was. The signal is still the single source of truth.
- **Why `prevent_default` matters more now.** In an `<input>`, Enter's default action
  inserted nothing, so `prevent_default` was a formality. In a `<textarea>`, Enter's
  default action is to insert `\n`. Without `prevent_default`, a sent message would
  leave a stray newline in the freshly cleared box: `submit` sets `draft` to `""`, then the
  browser applies the keypress. Shift+Enter skips the `if`, so the default applies and
  inserts the newline, and no code is needed for that case.
- **Why `is_composing`.** With an IME, the keydown for the Enter that *commits* a
  candidate still reports `key == Enter`. Without the check, a Japanese user sends
  half-typed text every time they confirm a word. It is one boolean, and forgetting it is
  a classic chat-box bug, which is why it gets a named test.
- **Why a free function and not a closure.** `sends` has no signals and no DOM, only three
  values in and a `bool` out. Pulled out of the handler, it can be tested with
  `cargo test` instead of by pressing keys. The handler is left with the plumbing:
  read the event, call `sends`, act on the answer. This is the same split as
  `Turn::of_swipe` in `reader.rs`.
- **`e.modifiers()`** comes from the `ModifiersInteraction` trait, which the Dioxus prelude
  brings into scope. If the compiler says there is no method `modifiers`, add
  `use dioxus::html::ModifiersInteraction;`. `Modifiers::shift()` is a
  `keyboard_types` convenience for `contains(Modifiers::SHIFT)`.

- **Why `white-space: pre-wrap` on the turn.** HTML's default `white-space: normal`
  collapses every run of whitespace, newlines included, into one space. So a turn
  flattens in the `li` just as it did in the `<input>`, but for a different reason:
  the text is intact and only its rendering collapses it. `pre-wrap` keeps the newlines
  and still wraps long lines. This step owns the fix because it is the first to put a
  newline into a turn. Gemini's replies benefit too, since they often have paragraphs.

**Scope note.** The passage is still fake until Step 4. Replies are shown as plain text:
Gemini's Markdown (`**bold**`, lists) appears as raw characters, and rendering Markdown is
not in this phase. Focusing the box on *Ask AI* is still unplanned.

## Step 3 — The author

> **Status:** done — committed in `21b4f65` (178 tests green, 2 ignored; clippy clean; `dx serve` checked by eye).

**What it is.** `OpenBook` carries the book's `author: Option<String>` from the `books` row
it was opened from, and *Ask AI* passes it into the `Passage`, so the draft reads
`I'm reading *<title>* by <author>, …`. Written ahead of the plan. It was Step 4 until
the swap.

**Check first: `dx serve`, eyeball.** Open a book that has an author and press *Ask AI*:
the header reads `by <author>`. Open one without an author (the library card shows no
author line) and press it again: the header goes straight from the title to the chapter,
with no stray `by`. That second case is `prompt::draft`'s job and is already under
`missing_author_and_chapter_leave_no_holes`. This check only confirms the `None` gets there.

**Minimal implementation.** `author: Option<String>` on `OpenBook` in `ui/library.rs`,
filled from `book.author` where the library card builds the `OpenBook`; in `Reader`, clone
it next to `title` and pass `author: author.as_deref()`.

**Why it works.** `Passage` borrows (`Option<&str>`), `OpenBook` owns
(`Option<String>`). `as_deref()` is the bridge: it turns `&Option<String>` into
`Option<&str>` without cloning, the `Option` equivalent of `&String → &str`. The
library card's handler moves `book.author` into its closure the same way it already moves
`title`: each click builds a fresh `OpenBook`, so it clones from the closure's copy.

## Step 4 — The real selection

> **Status:** done — committed in `f30cfde` (179 tests green, 2 ignored; clippy clean; `dx serve` checks confirmed by eye).

> **Written by:** `lbb:next-implement`. The agent wrote the implementation and tests;
> they are reviewed by hand.

**What it is.** *Ask AI* stops drafting from a hard-coded sentence and drafts from what is
actually selected in the chapter. The click handler becomes `async`: it runs a one-line
script through `document::eval` that asks the reader controller for the active frame's
selection, `.await`s the string, and builds the draft from it. If nothing is selected,
the drawer opens and the draft is left alone.

**Check first (part 1): `cargo test ui::reader`.** Add this to `mod test` in
`src/ui/reader.rs`. It fails to compile until `SELECTED_TEXT_JS` exists, then fails on
the controller half until `selectedText` is written:

```rust
#[test]
fn ask_ai_and_the_controller_agree_on_how_to_read_the_selection() {
    // Rust names a JS method in a string; nothing checks that the method exists.
    // Rename it on one side and Ask AI silently drafts nothing.
    assert!(SELECTED_TEXT_JS.contains("__ookReader?.selectedText()"));
    assert!(READER_CONTROLLER_JS.contains("selectedText()"));
    assert!(READER_CONTROLLER_JS.contains("getSelection()"));
}
```

**Check first (part 2): `dx serve`, eyeball.**

1. Select a sentence in the chapter and press *Ask AI*. The draft quotes **that sentence**.
2. Select across a paragraph break. Each paragraph is quoted on its own `> ` line, with
   no `> ` line at the very start or end.
3. Click in the text to clear the selection and press *Ask AI*. The drawer opens, and the
   draft from step 2 is still there, untouched.
4. Turn to the next chapter, select something there, and press *Ask AI*. The quote comes
   from the new chapter, not the one before. This checks that the method reads the active
   slot and not a preloaded one.
5. `cargo clippy --all-targets` is clean.

**Minimal implementation.**

`src/web/assets/host/reader/reader-controller.js`: a method on the `reader` object, next to
`resolveGesture`:

```js
selectedText() {
  return (
    this.slots[this.active].frame.contentWindow?.getSelection()?.toString() ?? ""
  );
},
```

Also add a line to the `ReaderController` typedef: ` * @property {() => string} selectedText`.

`src/ui/reader.rs`: a constant next to `GESTURE_RESULT_JS`:

```rust
const SELECTED_TEXT_JS: &str = r#"return window.__ookReader?.selectedText() ?? "";"#;
```

and the *Ask AI* handler, which reads the selection before drafting:

```rust
onclick: {
    let title = book.title.clone();
    let author = book.author.clone();
    let chapter = chapter_label.clone();
    move |_| {
        let (title, author, chapter) = (title.clone(), author.clone(), chapter.clone());
        async move {
            let selected = document::eval(SELECTED_TEXT_JS)
                .join::<String>()
                .await
                .unwrap_or_default();
            let text = selected.trim();
            if !text.is_empty() {
                chat_draft.set(prompt::draft(&Passage {
                    title: &title,
                    author: author.as_deref(),
                    chapter: Some(&chapter),
                    text,
                }));
            }
            chat_open.set(true);
        }
    }
},
```

**Why it works.**

- **Why it has to be `async`.** Rust never sees the DOM. `document::eval` sends the
  script to the webview and returns immediately. The answer comes back later as a message,
  so reading it is an `.await`. Dioxus accepts an event handler that returns a future and
  spawns it on the component's scope, the same machinery as Phase 19's `spawn`, without
  spelling it out.
- **Why `return` in the script, and `.join::<String>()`.** Dioxus wraps an `eval` script
  in an async JS function. Its `return` value is serialized to JSON and becomes what
  `join` deserializes. `join` consumes the `Eval`, so it suits a one-shot question.
  `recv` is for streams of messages (the bridge's `while let … recv()` loop). `?? ""`
  on the JS side covers the reader controller not being mounted yet: the result is still a
  string, never `null`, so `join::<String>` cannot fail on a type mismatch.
- **Why the second round of clones inside the closure.** The closure is called once per
  click, but `async move` needs to **own** everything it uses, because the future may
  outlive the call that created it. It cannot take the closure's own `title`, since the
  next click needs it too. So each call clones from the closure's copies and moves those
  clones into that click's future. Two layers: the outer block clones out of the render,
  and the inner line clones out of the closure.
- **Why the `trim`.** `Selection.toString()` often carries a leading or trailing newline
  when a selection starts or ends at a paragraph edge. Untrimmed, `draft` would quote it
  as an empty `> ` line (check 2). `trim` returns a `&str` into `selected`, so nothing is
  copied. `selected` has to live in its own `let` for that borrow to have something to
  point at.
- **Why read from the controller and not from `document.querySelector("iframe")`.** Two
  iframes are always mounted, the active one and a preloaded one. Only the controller
  knows which slot is `active`. Asking it keeps that knowledge in one place (check 4).
  The host can call `getSelection()` on the frame's window at all because the chapter is
  a `blob:` URL the host created, inside a sandbox with `allow-same-origin`, so the two
  documents share an origin.

**Scope note.** The button is still always visible, even with nothing selected. Step 5
hides it until there is a selection. The empty-selection branch stays anyway, for the
moment between the selection clearing and the click arriving. The selection is read as
plain text: images, footnote markers and ruby annotations come through as whatever
`toString()` makes of them. The 4,000-character cap in `prompt::draft` already bounds a
long selection, and says so with `...`.

## Between Steps 4 and 5 — Send on open (manual changes)

> **Status:** done — committed in `4218631` (179 tests green, 2 ignored; clippy clean;
> the send-on-open interaction is a `dx serve` check, done by hand).

> **Written by:** the learner, outside the step plan. The agent reviewed it.

**What it is.** Pressing the chat icon with text selected now **sends** the drafted
question right away, instead of leaving it in the compose box. The *Ask AI* text button
became the toolbar's chat icon (`ChatPanel` dropped its own trigger and `show_controls`).
The compose area gained **Reset**, and closing the drawer resets the conversation too.
The drawer is wider, and its turn colours were restyled.

**Check: `dx serve`, eyeball.**

1. Select a passage and tap the chat icon. The drawer opens with the passage as a user
   turn, then `...`. The textarea is empty.
2. Tap the icon with nothing selected. The drawer opens and nothing is sent.
3. With no Gemini key, select and tap. The draft stays in the textarea, unsent.
4. Close the drawer while a reply is pending, then reopen it. The chat is empty, and no
   late answer shows up.

**Shape.**

```rust
use_effect(move || {
    if autosend() {
        autosend.set(false);
        submit();
    }
});
```

`autosend: Signal<bool>` is owned by `Reader` and passed in like `open` and `draft`.
`Reader` sets it only inside the `if !text.is_empty()` branch of the click handler.

**Why it works.**

- **Why a flag, and why `Reader` owns it.** Only the click knows that the draft came from
  a selection. `submit` needs `chat` and the provider, which `ChatPanel` owns. The flag is
  the smallest message between the two: `Reader` says "send this", and the panel decides
  how. A local `use_signal(|| true)` inside `ChatPanel` looked similar but did nothing.
  The effect fired once on mount with an empty draft, cleared the flag, and nothing ever
  raised it again.
- **Why clear the flag before `submit()`.** A `use_effect` subscribes to every signal it
  reads during a run. On a `true` run, `submit` reads `chat`, `draft` and `provider`, so
  `chat.settle()` re-runs the effect later. Clearing first makes that re-run see `false`
  and return. Subscriptions are rebuilt on every run, so it goes back to watching only
  `autosend`.
- **Why `submit` can be shared.** It captures only `Signal`s, which are `Copy`, so the
  closure is `Copy` too. The effect, `onkeydown` and the Send button each get their own
  copy.
- **Why the failure path is graceful.** When there is no provider, or `ask` refuses
  because a reply is pending, `submit` returns before clearing `draft`. The passage stays
  in the textarea.
- **Why keep the `Task`.** `spawn` returns a handle. Storing it lets `reset` `cancel()`
  the in-flight `complete`. Without that, a reply to the old conversation would `settle`
  into the fresh one. `take()` leaves `None` behind, so there is no separate `set(None)`.

**Found in review.** A draft of the close handler also did `autosend.set(true)`. With text
in the compose box, closing the drawer would have sent that text to Gemini as the first
turn of a fresh chat, with the drawer shut. It was removed before the commit.

**What this changes for Step 5.** The icon is now the only way into the chat, so hiding it
while nothing is selected would lock the drawer away. Step 5 needs a new shape: perhaps a
selection-only *Ask AI* affordance next to an always-visible chat icon. Or the flag could
change only the icon's label and behaviour. Decide that before starting it.
