use dioxus::prelude::*;

use crate::{
    ai::{gemini::Gemini, Message, Role},
    ui::components::icon::{self, Icon},
};

#[css_module("/src/ui/chat.css")]
struct Styles;

#[component]
pub(crate) fn ChatPanel(show_controls: bool) -> Element {
    let mut open = use_signal(|| false);
    let provider = use_context::<Signal<Option<Gemini>>>();
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

    rsx! {
        button {
            class: if show_controls { "icon-button" } else { "icon-button reader-control--hidden" },
            aria_label: "Chat",
            onclick: move |_| open.set(true),
            Icon {
                icon: icon::MESSAGE,
            }
        }

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
                    onclick: move |_| open.set(false),
                    Icon {
                        icon: icon::CLOSE,
                    }
                }

            }
            if provider.read().is_none() {
                p {
                    style: "padding: 1rem",
                    "Add a Gemini key in settings to chat."
                }
            } else {
                ul {
                    class: "{Styles::chat_panel__messages}",
                    for message in messages.read().iter() {
                        li {
                            class: if message.role() == Role::User { "{Styles::chat_panel__turn} {Styles::chat_panel__turn_user}" } else { "{Styles::chat_panel__turn}" },
                            "{message.text()}"
                        }
                    }
                }
                if messages.read().is_empty() {
                    p {
                        style: "padding: 1rem",
                        "No messages yet."
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
                        disabled: draft.read().is_empty(),
                        onclick: move |_| submit(),
                        "Send"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
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
}
