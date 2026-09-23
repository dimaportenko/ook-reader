use dioxus::{core::Task, prelude::*};

use crate::{
    ai::{gemini::Gemini, ChatProvider, Role},
    chat::{Conversation, Status},
    ui::components::icon::{self, Icon},
};

#[css_module("/src/ui/chat.css")]
struct Styles;

#[component]
pub(crate) fn ChatPanel(
    mut open: Signal<bool>,
    mut draft: Signal<String>,
    mut autosend: Signal<bool>,
) -> Element {
    let provider = use_context::<Signal<Option<Gemini>>>();
    let mut chat = use_signal(Conversation::default);
    let mut pending_task = use_signal(|| None::<Task>);

    let mut submit = move || {
        let Some(gemini) = provider.read().clone() else {
            return;
        };

        if !chat.write().ask(&draft.read()) {
            return;
        }
        draft.set(String::new());

        let history = chat.read().messages().to_vec();
        pending_task.set(Some(spawn(async move {
            let outcome = gemini.complete(&history).await;
            chat.write().settle(outcome);
            pending_task.set(None);
        })));
    };

    let mut reset = move || {
        if let Some(task) = pending_task.take() {
            task.cancel();
        }
        chat.set(Conversation::default());
    };

    use_effect(move || {
        if autosend() {
            autosend.set(false);
            submit();
        }
    });

    let provider = provider.read();
    let conversation = chat.read();

    rsx! {
        aside {
            class: "{Styles::chat_panel}",
            "data-state": if open() { "open" } else { "closed" },
            inert: if !open() { true },
            aria_label: "Chat",
            onkeydown: move |e| e.stop_propagation(),
            div {
                class: "{Styles::chat_panel__header}",
                span {
                    "Chat"
                }
                button {
                    class: "icon-button",
                    aria_label: "Close chat",
                    onclick: move |_| {
                        open.set(false);
                        reset();
                    },
                    Icon {
                        icon: icon::CLOSE,
                    }
                }

            }
            if provider.is_none() {
                p {
                    class: "{Styles::chat_panel__note}",
                    "Add a Gemini key in settings to chat."
                }
            } else {
                if conversation.messages().is_empty() {
                    p {
                        class: "{Styles::chat_panel__note}",
                        "No messages yet."
                    }
                }
                ul {
                    class: "{Styles::chat_panel__messages}",
                    aria_live: "polite",
                    for message in conversation.messages().iter() {
                        li {
                            class: "{Styles::chat_panel__turn}",
                            "data-role": if message.role() == Role::User { "user" } else { "assistant" },
                            "{message.text()}"
                        }
                    }
                    if *conversation.status() == Status::Waiting {
                        li {
                            class: "{Styles::chat_panel__turn}",
                            "..."
                        }
                    }
                }
                if let Status::Failed(text) = conversation.status() {
                    p {
                        class: "{Styles::chat_panel__error}",
                        role: "alert",
                        "{text}"
                    }
                }
                div {
                    class: "{Styles::chat_panel__compose}",
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
                    div {
                        class: "{Styles::chat_panel__compose_actions}",
                        button {
                            class: "{Styles::chat_panel__compose_action}",
                            disabled: conversation.messages().is_empty() || *conversation.status() != Status::Idle,
                            onclick: move |_| reset(),
                            "Reset"
                        }
                        button {
                            class: "{Styles::chat_panel__compose_action}",
                            disabled: draft.read().trim().is_empty(),
                            onclick: move |_| submit(),
                            "Send"
                        }
                    }
                }
            }
        }
    }
}

fn sends(key: &Key, shift: bool, composing: bool) -> bool {
    *key == Key::Enter && !shift && !composing
}

#[cfg(test)]
mod test {
    use dioxus::prelude::Key;

    use super::sends;

    const CHAT_CSS: &str = include_str!("chat.css");
    const SLIDE: &str = "0.2s";

    #[test]
    fn the_drawer_pays_its_own_safe_area_because_fixed_escapes_the_body_box() {
        assert_eq!(
            CHAT_CSS.matches("env(safe-area-inset-").count(),
            3,
            "top, right and bottom touch the screen edge; the left edge is over the page",
        );
        assert!(!CHAT_CSS.contains("safe-aria"));
    }

    #[test]
    fn the_drawer_stays_visible_for_the_whole_slide_out() {
        let closed = CHAT_CSS
            .split_once(".chat_panel {")
            .expect("the closed state is the base rule")
            .1
            .split_once('}')
            .expect("an unclosed rule")
            .0;

        assert!(closed.contains(&format!("transform {SLIDE}")));
        assert!(
            closed.contains(&format!("visibility 0s linear {SLIDE}")),
            "a shorter delay hides the drawer mid-slide; a longer one leaves it \
             hit-testable off-screen",
        );
        assert!(CHAT_CSS.contains("@media (prefers-reduced-motion: reduce)"));
    }

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
}
