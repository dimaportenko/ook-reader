use dioxus::prelude::*;

use crate::settings::{font::FontFamily, Settings};

#[component]
pub(crate) fn FontFamilyPicker() -> Element {
    let mut settings = use_context::<Signal<Settings>>();

    rsx! {
        div {
            select {
                onchange: move |event| {
                    let slug = event.data.value();
                    settings.write().font_family = FontFamily::from_slug(&slug);
                },
                for opt in FontFamily::ALL {
                    option {
                        key: "{opt.slug()}",
                        value: opt.slug(),
                        selected: settings().font_family == opt,
                        {opt.slug()}
                    }
                }
            }
        }
    }
}
