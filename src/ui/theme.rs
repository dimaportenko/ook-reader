use dioxus::prelude::*;

use crate::{
    settings::{theme::Theme, Settings},
    ui::components::picker::SlugPicker,
};

#[component]
pub(crate) fn ThemePicker() -> Element {
    let mut settings = use_context::<Signal<Settings>>();

    rsx! {
        SlugPicker {
            label: "Theme",
            options: Theme::ALL.iter().map(|opt| (opt.slug(), opt.label())).collect::<Vec<_>>(),
            selected: settings().theme.slug(),
            on_pick: move |slug: String| settings.write().theme = Theme::from_slug(&slug),
        }
    }
}
