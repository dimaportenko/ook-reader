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
            Icon {
                icon: icon::MESSAGE,
            }
        }

        aside {
            class: "{Styles::chat_panel}",
            "data-state": if open() { "open" } else { "closed" },
            inert: if !open() { true },
            aria_label: "Chat",
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
                    "Add a Gemini key in settings to chat."
                }
            } else {
                p {
                    "No messages yet."
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
