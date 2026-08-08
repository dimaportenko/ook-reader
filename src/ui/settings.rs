use dioxus::prelude::*;

use crate::web::settings::{
    Settings, FONT_SIZE_MAX, FONT_SIZE_MIN, LINE_HEIGHT_MAX, LINE_HEIGHT_MIN,
};

#[component]
pub(crate) fn FontSizeControl() -> Element {
    let mut settings = use_context::<Signal<Settings>>();

    rsx! {
        div {
            button {
                disabled: settings().font_size <= FONT_SIZE_MIN,
                onclick: move |_| settings.write().zoom_out(),
                "A-"
            }
            span {
                style: "padding: 0 0.5rem",
                "{settings().font_size}%"
            }
            button {
                disabled: settings().font_size >= FONT_SIZE_MAX,
                onclick: move |_| settings.write().zoom_in(),
                "A+"
            }
        }
    }
}

#[component]
pub(crate) fn LineHeightControl() -> Element {
    let mut settings = use_context::<Signal<Settings>>();
    let leading = settings().line_height_css();

    rsx! {
        div {
            button {
                disabled: settings().line_height <= LINE_HEIGHT_MIN,
                onclick: move |_| settings.write().tighter(),
                "\u{2195}-"
            }
            span {
                style: "padding: 0 0.5rem",
                "{leading}"
            }
            button {
                disabled: settings().line_height >= LINE_HEIGHT_MAX,
                onclick: move |_| settings.write().looser(),
                "\u{2195}+"
            }
        }
    }
}
