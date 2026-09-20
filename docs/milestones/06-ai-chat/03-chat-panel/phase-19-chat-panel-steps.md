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

`src/ui/chat.rs` — a `ChatPanel` component: `use_signal(|| false)` for `open`, a
`button.icon-button` with an `aria_label: "Chat"` toggling it, and when open a
`div.chat-panel` (`position: fixed; right: 0; top: 0; bottom: 0; width: min(28rem, 100%)`)
holding a header row (title + close button) and the body. The body is a `match` on
`use_context::<Signal<Option<Gemini>>>()`: `None` → the "add a key" `p`; `Some(_)` →
`p { "No messages yet." }`.

`src/ui/reader.rs` — `ChatPanel {}` after `SettingsPopover {}` in the control row.

`src/ui/mod.rs` — `pub mod chat;`. Icon: `components::icon` has no chat glyph yet; add a
`MESSAGE` `TablerIcon` next to `SETTINGS` (Tabler's `message-circle` path).

Safe-area: `assets/main.css` pads `body` with `env(safe-area-inset-*)`, but a
`position: fixed` drawer escapes that box, so it needs its own
`padding-top: env(safe-area-inset-top)` / `-bottom` — the same lesson `toc.css` learned
for the popover's `right`.

**Why it works.**

- **A fixed `div`, not `PopoverRoot`.** The popovers anchor to their trigger and close on
  outside click; a chat needs to stay open while the user scrolls the page behind it and
  taps into the input. A plain fixed element with its own `open` signal is the smallest
  thing that behaves like a drawer.
- **The provider signal is read, never cloned, here.** `provider.read().is_some()` is a
  synchronous borrow inside render, released before the function returns. Step 4 is the
  first place a `.await` appears, and the clone-before-spawn rule applies there, not yet.
- **The `None` branch is in Step 1 on purpose.** It makes the shell observable in two
  states from one signal that already exists, and it is the state a fresh install lands in.
- **`show_controls` hides the button, not the drawer.** The drawer is a separate surface;
  hiding it with the chrome would drop a half-typed question on a stray tap.

**Scope.** No list, no input, no `chat` module. Step 2 adds the list and input on a bare
`Vec<Message>` signal.

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

`src/ai/mod.rs` — the UI will need to read a turn without owning it. Add to `impl Message`:

```rust
pub(crate) fn role(&self) -> Role {
    self.role
}

pub(crate) fn text(&self) -> &str {
    &self.text
}
```

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

`ui/chat.rs` — the rewire: `use_signal(Vec::new)` seeded in Step 2 becomes
`use_signal(Conversation::default)`; the list iterates `chat.read().messages()`; submit
becomes `if chat.write().ask(&draft()) { draft.set(String::new()) }`. The seed turns and
the push-on-submit both go, because the struct now owns those transitions.

**Why it works.**

- **`ask` returns `bool`, not `Result`.** A blank line and a double-submit are not errors
  the reader should see; they are inputs the state machine declines. The drawer uses the
  `bool` to decide whether to clear the input. Reserve `Status::Failed` for the
  one thing that *is* an error: the provider said no.
- **`settle` takes `Result<Reply, ChatError>` by value** — exactly what `complete` returns,
  so Step 5's `spawn` body is one line: `chat.write().settle(outcome)`. It stores
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
