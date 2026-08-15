use dioxus::prelude::*;

use crate::settings::{theme::Theme, Settings};

#[component]
pub(crate) fn ThemePicker() -> Element {
    let mut settings = use_context::<Signal<Settings>>();

    rsx! {
        div {
            select {
                onchange: move |event| {
                    let slug = event.data.value();
                    settings.write().theme = Theme::from_slug(&slug);
                },
                for opt in [Theme::Day, Theme::Sepia, Theme::Night] {
                    option {
                        key: "{opt.slug()}",
                        value: opt.slug(),
                        selected: settings().theme == opt,
                        {opt.slug()}
                    }
                }
            }
        }
    }
}
