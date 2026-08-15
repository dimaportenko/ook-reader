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
            options: Theme::ALL.iter().map(|opt| opt.slug()).collect::<Vec<_>>(),
            selected: settings().theme.slug(),
            on_pick: move |slug: String| settings.write().theme = Theme::from_slug(&slug),
        }
    }
}
