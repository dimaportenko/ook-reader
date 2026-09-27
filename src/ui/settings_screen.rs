use dioxus::prelude::*;

use crate::ui::components::icon::{self, Icon};

#[css_module("/src/ui/settings_screen.css")]
struct Styles;

#[component]
pub(crate) fn SettingsScreen(mut open: Signal<bool>) -> Element {
    if !open() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "{Styles::settings_screen}",
            role: "dialog",
            aria_label: "Settings",
            header {
                class: "{Styles::settings_screen__header}",
                h1 { "Settings" }
                button {
                    class: "icon-button",
                    aria_label: "Close settings",
                    onclick: move |_| open.set(false),
                    Icon { icon: icon::CLOSE }
                }
            }
        }
    }
}
