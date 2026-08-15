use dioxus::prelude::*;

use crate::{
    settings::{font::FontFamily, Settings},
    ui::components::picker::SlugPicker,
};

#[component]
pub(crate) fn FontFamilyPicker() -> Element {
    let mut settings = use_context::<Signal<Settings>>();

    rsx! {
        SlugPicker {
            options: FontFamily::ALL.iter().map(|opt| opt.slug()).collect::<Vec<_>>(),
            selected: settings().font_family.slug(),
            on_pick: move |slug: String| settings.write().font_family = FontFamily::from_slug(&slug),
        }
    }
}
