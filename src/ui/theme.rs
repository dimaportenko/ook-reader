use dioxus::prelude::*;

use crate::web::theme::Theme;

#[component]
pub(crate) fn ThemePicker() -> Element {
    let mut theme = use_context::<Signal<Theme>>();

    rsx! {
        div {
            select {
                onchange: move |event| {
                    let slug = event.data.value();
                    theme.set(Theme::from_slug(&slug));
                },
                for opt in [Theme::Day, Theme::Sepia, Theme::Night] {
                    option {
                        key: "{opt.slug()}",
                        value: opt.slug(),
                        selected: theme() == opt,
                        {opt.slug()}
                    }
                }
            }
        }
    }
}
