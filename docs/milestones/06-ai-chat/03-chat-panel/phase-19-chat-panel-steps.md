# Phase 19 — Chat panel — build log

[← Phase 19](phase-19-chat-panel.md) · the phase doc holds the goal, decisions and the
step index; this file holds the test → code → why for each step, newest at the bottom.

## The crux

The network call is the first `.await` in the UI, and a component cannot wait. Event
handler → `spawn` → `.await` the provider → write a signal → the render that shows it. Never
hold a signal's read guard across the `.await`; clone the provider out first. And keep the
*asked → waiting → answered | failed* machine on a plain struct so it is testable without
Dioxus.

## Step plan

1. The drawer shell — chat button in the control row + `ChatPanel`, "add a key in settings" on `None`; iOS geometry check.
2. The message list and input — `Signal<Vec<Message>>` seeded with two turns; submit appends, no network.
3. The conversation state — `Conversation::ask` / `settle`, `Status`; accessors on `Message`; the drawer rewired onto it, seed removed.
4. The async send — `spawn` + `settle` straight through `provider.complete`, waiting and error rows; desktop then iOS.
5. Review and refactor.

**Why this order.** The first draft went state-first (struct → send helper → drawer → list →
async). Re-ordered 2026-09-20 so every step ends with something to click: a struct with
no caller is only checkable by its tests, while a drawer with a fake list is checkable by
eye on desktop and on the simulator, where the safe-area bugs live. The struct arrives in
Step 3, right after the submit that needs its double-submit guard, so the refactor from a
bare `Vec` onto `Conversation` is visible as its own diff. The `chat::send` helper was
dropped: `complete` already takes the full slice, and there is no history logic to test
until Phase 22 trims it.

## Step 1 — The drawer shell

**What it is.** A third control beside the contents and settings popovers that opens a
`ChatPanel` drawer over the reader. The drawer has a title, a close button, and one line of
body: "Add a Gemini key in settings to chat" when the provider signal in context is `None`,
otherwise a placeholder line that Step 2 replaces with the list. Nothing is sent, nothing
is stored.

**Check first — `dx serve`, eyeball.** Open a book, tap the new chat icon: the drawer
slides in over the page, the page stays visible behind it, the close button dismisses it,
and the tap-to-hide-controls rule hides the chat button with the others. Forget the key in
settings: the body switches to the "add a key" line without reopening. Then
`cargo clippy --all-targets` for the unused-signal warnings a shell tends to leave behind.

**Second check — the iOS simulator.** Build with `dx build --platform ios`, open the app
through `agent-device`, open the drawer, and read the `rect`s from
`agent-device snapshot -i --json`: the drawer's close button must sit inside the safe area
on the iPhone, and the drawer must not extend under the home indicator. This is the point
to catch it, before there is a list to scroll inside it.

**Minimal implementation.**

`src/ui/components/icon.rs`, next to `SETTINGS`:

```rust
pub(crate) const MESSAGE: TablerIcon = TablerIcon {
    name: "message-circle",
    paths: &[
        "M3 20l1.3 -3.9c-2.324 -3.437 -1.426 -7.872 2.1 -10.374c3.526 -2.501 8.59 -2.296 11.845 .48c3.255 2.777 3.695 7.266 1.029 10.501c-2.666 3.235 -7.615 4.215 -11.574 2.293l-4.7 1",
    ],
};
```

`src/ui/chat.rs`, new file, plus `pub mod chat;` in `ui/mod.rs`:

```rust
use dioxus::prelude::*;

use crate::{
    ai::gemini::Gemini,
    ui::components::icon::{self, Icon},
};

#[css_module("/src/ui/chat.css")]
struct Styles;

#[component]
pub(crate) fn ChatPanel(show_controls: bool) -> Element {
    let mut open = use_signal(|| false);
    let provider = use_context::<Signal<Option<Gemini>>>();

    rsx! {
        button {
            class: if show_controls { "icon-button" } else { "icon-button reader-control--hidden" },
            aria_label: "Chat",
            onclick: move |_| open.set(true),
            Icon { icon: icon::MESSAGE }
        }
        if open() {
            aside {
                class: "{Styles::chat_panel}",
                aria_label: "Chat",
                div {
                    class: "{Styles::chat_panel__header}",
                    span { "Chat" }
                    button {
                        class: "icon-button",
                        aria_label: "Close chat",
                        onclick: move |_| open.set(false),
                        Icon { icon: icon::CLOSE }
                    }
                }
                if provider.read().is_none() {
                    p { "Add a Gemini key in settings to chat." }
                } else {
                    p { "No messages yet." }
                }
            }
        }
    }
}
```

`src/ui/chat.css`, new file; the colour pair is the one `components/popover/style.css`
uses for its content surface:

```css
.chat-panel {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  width: min(28rem, 100%);
  padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) 0;
  background: var(--light, var(--primary-color)) var(--dark, var(--primary-color-5));
  box-shadow: -2px 0 12px rgb(0 0 0 / 0.2);
  z-index: 2;
}

.chat-panel__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 1rem;
}
```

`src/ui/reader.rs`, in the control row after `SettingsPopover {}`:

```rust
ChatPanel { show_controls: show_controls() }
```

**Why it works.**

- **A fixed `aside`, not `PopoverRoot`.** The popovers anchor to their trigger and close on
  outside click; a chat needs to stay open while the user scrolls the page behind it and
  taps into the input. A plain fixed element with its own `open` signal is the smallest
  thing that behaves like a drawer.
- **`provider.read().is_none()` inside `rsx!` is safe.** It is a synchronous borrow during
  render, released before the function returns — and reading it subscribes the component,
  so when settings forgets the key and writes `None`, this component re-renders and the
  body line flips. Step 4 is the first place a `.await` appears, and the clone-before-spawn
  rule applies there, not yet.
- **`show_controls` is a prop, not a context read,** matching `NavRow`. The prop hides the
  button only. The drawer is a separate surface; hiding it with the chrome would drop a
  half-typed question on a stray tap once Step 2 adds the input.
- **The `None` branch is in Step 1 on purpose.** It makes the shell observable in two
  states from one signal that already exists, and it is the state a fresh install lands in.
- **The drawer sets its own safe-area padding.** `assets/main.css` pads `body`, but
  `position: fixed` escapes that box — the same lesson `toc.css` learned for the popover's
  `right`.
- **`if open() { aside { … } }` unmounts the drawer** rather than hiding it. Cheaper than a
  CSS toggle, and Step 2's list state will live in a signal owned by `ChatPanel`, so it
  survives a close and reopen regardless.

**Slide in, slide out (folded in while building).** `if open() { aside }` cannot animate
out — the element is gone the same frame `open` flips. So the drawer stays mounted and
animates on a `data-state` attribute, the pattern the popover already uses: `transform:
translateX(100%)` closed, `translateX(0)` open, a `0.2s` transition on `transform`, and
`visibility: hidden` delayed by the same `0.2s` so the closed drawer is neither visible nor
hit-testable. `inert` removes it from focus and the accessibility tree while closed.

**Found on the way: `inert: !open()` made the drawer inert in both states.** Dioxus 0.7.9's
interpreter keeps a hard-coded list of boolean attributes and `inert` is not on it, so
`false` renders `inert="false"`, which the browser reads as inert. Fix: `inert: if !open()
{ true }` — a conditional attribute with no `else` is omitted entirely. SSR's list already
has `inert`; a fix PR to Dioxus is drafted separately.

**Tests added at commit time.** Two of the five review findings were misspelt CSS tokens
(`safe-aria-inset-top`, `prefers-reduce-motion`) that no desktop eyeball catches, so
`ui::chat::test` pins the inset spelling and count, and the visibility delay against the
slide duration. Watched to fail by mutating the expected duration.

**Scope.** No list, no input, no `chat` module. Step 2 adds the list and input on a bare
`Vec<Message>` signal.

> **Status:** done — committed in `c8e2488` (169 tests green). Desktop eyeball confirmed
> by the learner; the iOS geometry check is still owed and runs before Step 2 lands.

## Step 2 — The message list and input

**What it is.** The drawer's body becomes a list of turns and a one-line input with a Send
button. The list is a `Signal<Vec<Message>>` seeded with one user turn and one assistant
turn, so both roles are on screen from the first render and the CSS for each can be tuned
against something real. Submit — Enter in the input, or the button — appends a trimmed
user turn and clears the input. Nothing is sent; the list only ever grows by user turns.

**Check first — `dx serve`, eyeball.** Open a book, open the drawer:

- Two seeded turns are visible, user right-aligned (or otherwise visibly distinct from)
  assistant, both readable in light and dark theme.
- Type `hello`, press Enter: a third bubble appears at the bottom, the input is empty.
  Click Send after typing: same result.
- Press Enter on a blank or whitespace-only input: nothing appends.
- With the caret in the input, press `←` / `→` / `Space`: the page behind **must not turn**
  and the caret must move as normal (see the propagation note under *Why*).
- Forget the key in settings: the list and input give way to the "add a key" line; add it
  back and the seeded turns are still there (the signal survived).

Then `cargo clippy --all-targets`, and `cargo test` stays at 169 — nothing here is
unit-testable; the trim rule gets its `#[test]` when Step 3 moves it onto `Conversation`.

**Minimal implementation.**

`src/ai/mod.rs`, in `impl Message` — pulled forward from Step 3 because the list cannot
read a turn's private fields without them:

```rust
pub(crate) fn role(&self) -> Role {
    self.role
}

pub(crate) fn text(&self) -> &str {
    &self.text
}
```

`src/ui/chat.rs` — new signals under `provider`, and the body replaces the "No messages
yet." paragraph:

```rust
let mut messages = use_signal(|| {
    vec![
        Message::user("Which city is this set in?"),
        Message::assistant("Ankh-Morpork."),
    ]
});
let mut draft = use_signal(String::new);

let mut submit = move || {
    let question = draft.read().trim().to_owned();
    if question.is_empty() {
        return;
    }
    messages.write().push(Message::user(question));
    draft.set(String::new());
};
```

```rust
aside {
    // …existing attributes…
    onkeydown: move |e| e.stop_propagation(),
    div { class: "{Styles::chat_panel__header}", /* unchanged */ }
    if provider.read().is_none() {
        p { style: "padding: 1rem", "Add a Gemini key in settings to chat." }
    } else {
        ul {
            class: "{Styles::chat_panel__messages}",
            for message in messages.read().iter() {
                li {
                    class: if message.role() == Role::User {
                        "{Styles::chat_panel__turn} {Styles::chat_panel__turn_user}"
                    } else {
                        "{Styles::chat_panel__turn}"
                    },
                    "{message.text()}"
                }
            }
        }
        div {
            class: "{Styles::chat_panel__compose}",
            input {
                value: "{draft}",
                placeholder: "Ask about this book",
                oninput: move |e| draft.set(e.data.value()),
                onkeydown: move |e| {
                    if e.key() == Key::Enter {
                        e.prevent_default();
                        submit();
                    }
                },
            }
            button {
                disabled: draft.read().trim().is_empty(),
                onclick: move |_| submit(),
                "Send"
            }
        }
    }
}
```

`use crate::ai::{Message, Role};` joins the imports.

`src/ui/chat.css` — the drawer becomes a column so the list takes the slack and the
compose row sits at the bottom, above the home indicator (the `padding` already pays the
bottom inset):

```css
.chat_panel {
  /* …existing… */
  display: flex;
  flex-direction: column;
}

.chat_panel__messages {
  flex: 1;
  overflow-y: auto;
  margin: 0;
  padding: 0 1rem;
  list-style: none;
}

.chat_panel__turn {
  max-width: 85%;
  margin: 0.5rem 0;
  padding: 0.5rem 0.75rem;
  border-radius: 0.75rem;
  background: color-mix(in srgb, var(--USER__backgroundColor) 80%, black);
}

.chat_panel__turn_user {
  margin-left: auto;
  background: var(--primary-color-4);
}

.chat_panel__compose {
  display: flex;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
}

.chat_panel__compose input {
  flex: 1;
}
```

**Why it works.**

- **`submit` is a closure, not a nested component or a second copy.** Enter and the button
  are two events that mean the same thing; one `move ||` closure holding the two signal
  handles (`Signal` is `Copy`, so `move` copies handles, not the data) keeps the trim rule
  in exactly one place. Calling it from two handlers needs it to be `FnMut`, hence
  `let mut submit`.
- **`draft.read().trim().to_owned()` then `draft.set(…)` — in that order.** The `read()`
  guard is a temporary that dies at the end of its statement, so by the time `set` runs the
  borrow is gone. Writing `draft.set` *while* the guard is alive would panic at runtime,
  the same rule the crux states for `.await`, just in miniature.
- **`for … in messages.read().iter()` inside `rsx!`** subscribes the component to
  `messages`; the `push` inside `submit` is a write, which schedules the re-render that
  shows the new bubble. Nothing else needs to tell the list to refresh.
- **`onkeydown: stop_propagation` on the `aside`** is the same fix `popover/component.rs`
  applies: `reader-root` turns pages on arrow keys and Space, and a keydown inside the
  input bubbles up to it. Stopping the event at the drawer keeps typing local. Without it
  the eyeball check above fails — and it is the kind of bug a screenshot never shows.
- **`prevent_default` on Enter** stops the browser's own handling (a beep on macOS
  WebKit, an implicit form submit had this been a `form`). The `Key::Enter` pattern is the
  one the `dioxus-07` skill documents; a `form { onsubmit }` would also work but adds a
  default-action to reason about for no gain here.
- **`disabled: draft.read().trim().is_empty()`** mirrors the Save button in settings — the
  same rule as the early `return` in `submit`, expressed a second time so the UI *shows*
  it. Step 3 makes it a single source of truth by moving the rule into `Conversation::ask`.
- **The seed is deliberate scaffolding.** Two real `Message` values, not lorem-ipsum
  strings, so the list is built against the type the provider actually returns; Step 3
  deletes the seed when `Conversation::default()` takes over.
- **`if`/`else`, not `match`, for the class.** `rsx!` only interpolates `"{…}"` literals
  that sit directly in attribute position, and `if`/`else` there is special-cased; a
  `match` arm's literal is a plain `&str` and lands in the DOM verbatim as
  `{Styles::…}`. Found while building — a `match` would need `format!`.
- **Class names use underscores.** `css_module` maps each CSS class to a Rust identifier,
  so `chat_panel__turn_user` rather than a BEM `--user` modifier, matching `chat_panel`.

**Scope.** No double-submit guard, no waiting or error row, no scroll-to-bottom on append,
no `chat` module. Step 3 lifts the trim/append logic onto `Conversation` with tests; Step
4 adds the network and the waiting row. The iOS geometry check owed from Step 1 runs on
this build, now that there is a compose row to sit above the home indicator.

**Found on the way: `match` does not interpolate.** The first draft chose the bubble class
with a `match` on the role; the literal reached the DOM as `{Styles::chat_panel__turn}`.
Switched to `if`/`else`, which `rsx!` special-cases in attribute position.

> **Status:** done — committed in `2352a77` (169 tests green, none new — the step planned
> no unit test; clippy clean). Desktop eyeball by the learner. Still owed: the iOS
> geometry check from Step 1, now against the compose row.

## Step 3 — The conversation state

**What it is.** A `Conversation` value that knows the turns so far and whether it is waiting
on the provider. Two methods drive every transition the drawer will ever make: `ask` records
a user turn and enters *waiting*; `settle` takes the provider's `Result` and either appends
the assistant turn or records the failure. No Dioxus, no network — this step is pure Rust,
checked with `cargo test`.

**Check first — `cargo test chat::`.** Create `src/chat/mod.rs` with just the test module
below and `mod chat;` in `main.rs`; it fails to compile, which is the red. Then write the
struct until it is green. Second check, `dx serve`: the drawer from Step 2 now opens empty,
submit still appends the turn, and a second submit before any reply is ignored.

```rust
#[cfg(test)]
mod test {
    use super::*;
    use crate::ai::{ChatError, Message, Reply, Role};

    fn reply(text: &str) -> Result<Reply, ChatError> {
        Ok(Reply {
            text: text.to_owned(),
        })
    }

    #[test]
    fn asking_records_the_turn_and_waits() {
        let mut chat = Conversation::default();

        assert!(chat.ask("Which city?"));

        assert_eq!(chat.messages(), &[Message::user("Which city?")]);
        assert_eq!(chat.status(), &Status::Waiting);
    }

    #[test]
    fn a_blank_question_is_not_a_turn() {
        let mut chat = Conversation::default();

        assert!(!chat.ask("  \n"));

        assert!(chat.messages().is_empty());
        assert_eq!(chat.status(), &Status::Idle);
    }

    #[test]
    fn a_question_is_stored_trimmed() {
        let mut chat = Conversation::default();

        chat.ask("  Which city?\n");

        assert_eq!(chat.messages()[0].text(), "Which city?");
        assert_eq!(chat.messages()[0].role(), Role::User);
    }

    #[test]
    fn a_reply_lands_as_an_assistant_turn() {
        let mut chat = Conversation::default();
        chat.ask("Which city?");

        chat.settle(reply("Ankh-Morpork"));

        assert_eq!(
            chat.messages(),
            &[
                Message::user("Which city?"),
                Message::assistant("Ankh-Morpork")
            ]
        );
        assert_eq!(chat.status(), &Status::Idle);
    }

    #[test]
    fn a_failure_keeps_the_question_so_it_can_be_asked_again() {
        let mut chat = Conversation::default();
        chat.ask("Which city?");

        chat.settle(Err(ChatError::Empty));

        assert_eq!(chat.messages(), &[Message::user("Which city?")]);
        assert_eq!(
            chat.status(),
            &Status::Failed("the provider returned no answer".to_owned())
        );
        assert!(chat.ask("Which city, again?"), "a failed chat accepts a new turn");
    }

    #[test]
    fn asking_while_waiting_is_refused() {
        let mut chat = Conversation::default();
        chat.ask("First?");

        assert!(!chat.ask("Second?"));

        assert_eq!(chat.messages().len(), 1);
    }
}
```

**Minimal implementation.**

`src/ai/mod.rs` — the `role()` / `text()` accessors already landed in Step 2; nothing to add.

`src/chat/mod.rs`:

```rust
use crate::ai::{ChatError, Message, Reply};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum Status {
    #[default]
    Idle,
    Waiting,
    Failed(String),
}

#[derive(Debug, Default)]
pub(crate) struct Conversation {
    messages: Vec<Message>,
    status: Status,
}

impl Conversation {
    /// Records a user turn and enters `Waiting`. Returns `false`, and changes nothing,
    /// when the question is blank or a reply is still pending.
    pub(crate) fn ask(&mut self, question: &str) -> bool {
        let question = question.trim();
        if question.is_empty() || self.status == Status::Waiting {
            return false;
        }
        self.messages.push(Message::user(question));
        self.status = Status::Waiting;
        true
    }

    pub(crate) fn settle(&mut self, outcome: Result<Reply, ChatError>) {
        self.status = match outcome {
            Ok(reply) => {
                self.messages.push(Message::assistant(reply.text));
                Status::Idle
            }
            Err(error) => Status::Failed(error.to_string()),
        };
    }

    pub(crate) fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub(crate) fn status(&self) -> &Status {
        &self.status
    }
}
```

`main.rs`: `mod chat;` in the module list.

`ui/chat.rs` — the rewire against what Step 2 landed:

- `let mut messages = use_signal(|| vec![…seed…])` becomes
  `let mut chat = use_signal(Conversation::default)`.
- `submit` shrinks to `if chat.write().ask(&draft.read()) { draft.set(String::new()) }` —
  the trim, the blank check and the push all move into `ask`. Mind the guard order: the
  `draft.read()` temporary dies at the end of the `if` condition, before `draft.set` runs
  in the body.
- The list iterates `chat.read().messages().iter()`; the `if messages.read().is_empty()`
  paragraph now reads `chat.read().messages().is_empty()` — and it is finally reachable,
  since a fresh `Conversation` has no seed.
- `disabled: draft.read().trim().is_empty()` — the Send button's rule should match
  `ask`'s, so a whitespace draft neither enables the button nor makes a turn.
- `use crate::chat::{Conversation, Status};` — `Status` is unused until Step 4 renders it;
  leave it out until then so clippy stays quiet.

**Why it works.**

- **`ask` returns `bool`, not `Result`.** A blank line and a double-submit are not errors
  the reader should see; they are inputs the state machine declines. The drawer uses the
  `bool` to decide whether to clear the input. Reserve `Status::Failed` for the
  one thing that *is* an error: the provider said no.
- **`settle` takes `Result<Reply, ChatError>` by value** — exactly what `complete` returns,
  so Step 4's `spawn` body is one line: `chat.write().settle(outcome)`. It stores
  `error.to_string()` rather than the `ChatError` because the struct wants to be `Clone`
  and `PartialEq` for the signal and the tests, and `reqwest::Error` inside `ChatError` is
  neither. The `Display` text is what the drawer shows anyway.
- **The `Status::Waiting` guard is the double-submit lock.** Step 4 will `spawn` once per
  successful `ask`; refusing a second `ask` while waiting means there can never be two
  in-flight requests writing to one conversation. The invariant lives in the struct, so the
  component does not have to remember it.
- **`#[default]` on a variant** is the derive for enums: `Status::default()` is `Idle`, and
  `Conversation::default()` composes it, so `use_signal(Conversation::default)` needs no
  constructor.
- **Accessors over `pub` fields.** `messages()` returns `&[Message]`, not `&Vec<Message>`
  — the caller can iterate and index but cannot push around `ask`. That is the difference
  between a struct that *holds* a state machine and one that *is* one.

**Scope.** Nothing here calls a provider — the `Waiting` state is entered and left by the
tests directly, and in the running app a submit leaves the drawer in `Waiting` with nothing
to settle it. That is expected: Step 4 wires `ask` → `spawn` → `settle` and renders the
waiting row.

**Found on the way.** The first rewire dropped `ask`'s `bool` and always cleared the draft,
so a submit refused while `Waiting` threw the typed question away; fixed by clearing only
on `true`. Clippy then flagged `settle`, `status` and `Failed` as dead until Step 4, so
`mod chat` carries `#[allow(dead_code)]` like `mod ai`, to be removed in Step 4.

> **Status:** done — committed in `43e8a9e` (175 tests green, 6 new in `chat::test`,
> added at commit time; `asking_while_waiting_is_refused` watched to fail by inverting
> its assertion). Clippy clean.

## Step 4 — The async send

**What it is.** Submit now sends. After `ask` accepts the turn, the handler `spawn`s a task
that calls `complete` on the whole history and `settle`s the result. While the task runs
the list shows a waiting row; if it fails, an error row sits under the list with the
question still in place. This is the phase's crux made concrete: the component never
waits — it renders *waiting* now and *answered* later, both from one signal.

**Check first — `dx serve` with a real key, eyeball.** Save a Gemini key in settings, open
a book, open the drawer:

- Type a question, Enter: the user bubble appears, a waiting row ("…") appears under it,
  the input is cleared. A second Enter while waiting does nothing (that is `ask` refusing).
- A few seconds later the waiting row is replaced by an assistant bubble with a real answer.
- Break the key (settings → forget → save `nope`): ask again. The waiting row appears, then
  an error row reading `the provider rejected the request (400): …`. The question bubble
  is still there. Fix the key, ask again: it works, and the error row is gone.
- Close and reopen the drawer mid-wait: the answer still lands (the task is not tied to
  the drawer being open).

Then `cargo clippy --all-targets` — with the `#[allow(dead_code)]` on `mod chat` removed,
because everything in it now has a caller — and `cargo test` stays at 175.

**Second check — the iOS simulator.** `dx build --platform ios`, open via `agent-device`,
and drive the same flow with the key saved on the device. This is the end-to-end run the
phase's goal names, and it settles the geometry debt from Steps 1–2: read the compose
row's and close button's `rect`s from `agent-device snapshot -i --json` against the safe
area.

**Minimal implementation.**

`src/ai/gemini.rs` — the task needs its own `Gemini`, so the struct must be cloneable.
`reqwest::Client` is an `Arc` inside, so the clone is two `String`s and a refcount bump:

```rust
#[derive(Clone)]
pub(crate) struct Gemini { … }
```

`src/main.rs` — drop the `#[allow(dead_code)]` above `mod chat;`.

`src/ui/chat.rs` — imports gain `ChatProvider` (the trait must be in scope to call
`complete`) and `Status`:

```rust
use crate::{
    ai::{gemini::Gemini, ChatProvider, Role},
    chat::{Conversation, Status},
    …
};
```

`submit` grows the send:

```rust
let mut submit = move || {
    let Some(gemini) = provider.read().clone() else {
        return;
    };
    if !chat.write().ask(&draft.read()) {
        return;
    }
    draft.set(String::new());

    let history = chat.read().messages().to_vec();
    spawn(async move {
        let outcome = gemini.complete(&history).await;
        chat.write().settle(outcome);
    });
};
```

The rows, after the `for message in …` loop inside the `ul`, then after the `ul`:

```rust
if *chat.read().status() == Status::Waiting {
    li {
        class: "{Styles::chat_panel__turn}",
        aria_live: "polite",
        "…"
    }
}
```

```rust
if let Status::Failed(text) = chat.read().status() {
    p {
        class: "{Styles::chat_panel__error}",
        role: "alert",
        "{text}"
    }
}
```

`src/ui/chat.css`:

```css
.chat_panel__error {
  margin: 0;
  padding: 0.5rem 1rem;
  color: var(--error-color, #b00020);
}
```

**Why it works.**

- **Everything the task needs is owned before `spawn`.** `gemini` is a clone, `history` is
  a `to_vec()` — no `read()` guard is alive inside the `async move` block. The crux's rule
  exactly: a guard held across the `.await` would make `chat.write().settle(...)` — or any
  render that reads `chat` — panic on a borrow already taken. `chat` itself crosses into
  the block as a `Copy` handle, which is fine; it is the *guards* that must not.
- **`let … else` on the provider first.** If the key is gone, `submit` returns before `ask`
  runs, so no user turn is recorded that nothing will ever settle. Order matters: `ask` is
  the write that enters `Waiting`, and it must be the last check before the send.
- **`spawn` returns a `Task` we drop on purpose.** In Dioxus 0.7 a task spawned from a
  handler is owned by the component's scope and keeps running until it completes or the
  component unmounts; the drawer staying mounted while closed (Step 1) is why "close mid-wait"
  still lands the answer. Cancelling on re-ask is unnecessary — `ask` refuses while
  `Waiting`, so there is at most one task in flight per conversation.
- **`settle` runs on the main thread, after the await.** Dioxus's desktop and mobile
  runtimes are single-threaded executors: the `.await` suspends, the future resumes on the
  same thread, and `chat.write()` is an ordinary signal write that schedules a render. No
  `Arc<Mutex>`, no channel.
- **`history` is the whole list, on every request.** `generateContent` is stateless and the
  request body already takes the full `contents`; the copy is the price of not holding a
  borrow. Trimming is Phase 22's problem, as the phase doc records.
- **`Status` is matched in render, never `clone()`d into the closure.** The rows read
  `chat.read().status()` during render — a synchronous borrow that subscribes the
  component — so when `settle` writes, both rows flip in the same render as the new bubble.
- **`aria_live` / `role: "alert"`** are the difference between a sighted user seeing the
  answer land and a VoiceOver user hearing it. Cheap now; retrofitting is not.

**Scope.** No cancel button, no retry button (re-typing the question is the retry; a
one-tap retry is a Phase 22 nicety), no scroll-to-bottom on a new bubble, no disabled state
on the input while waiting — `ask` already refuses, and a greyed input would drop a
half-typed follow-up. No new unit test: the pure-Rust transitions were pinned in Step 3,
and the `Clone` derive has nothing to assert. Step 5 reviews the whole phase.
