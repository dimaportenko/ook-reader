use dioxus::prelude::*;
use dioxus_primitives::ContentAlign;

use crate::{
    settings::{
        Settings, FONT_SIZE_MAX, FONT_SIZE_MIN, LINE_HEIGHT_MAX, LINE_HEIGHT_MIN,
        MAX_LINE_LENGTH_MAX, MAX_LINE_LENGTH_MIN, PAGE_MARGINS_MAX, PAGE_MARGINS_MIN,
    },
    ui::{
        components::{
            icon::{self, Icon},
            popover::{PopoverContent, PopoverRoot, PopoverTrigger},
        },
        font::FontFamilyPicker,
        theme::ThemePicker,
    },
};

#[css_module("/src/ui/settings.css")]
struct Styles;

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

#[component]
pub(crate) fn PageMarginsControl() -> Element {
    let mut settings = use_context::<Signal<Settings>>();
    let margins = settings().page_margins_css();

    rsx! {
        div {
            button {
                disabled: settings().page_margins <= PAGE_MARGINS_MIN,
                onclick: move |_| settings.write().narrower(),
                "\u{2194}-"
            }
            span {
                style: "padding: 0 0.5rem",
                "{margins}"
            }
            button {
                disabled: settings().page_margins >= PAGE_MARGINS_MAX,
                onclick: move |_| settings.write().wider(),
                "\u{2194}+"
            }
        }
    }
}

#[component]
pub(crate) fn MaxLineLengthControl() -> Element {
    let mut settings = use_context::<Signal<Settings>>();

    rsx! {
        div {
            button {
                disabled: settings().max_line_length <= MAX_LINE_LENGTH_MIN,
                onclick: move |_| settings.write().shorter(),
                "\u{2261}-"
            }
            span {
                style: "padding: 0 0.5rem",
                "{settings().max_line_length}"
            }
            button {
                disabled: settings().max_line_length >= MAX_LINE_LENGTH_MAX,
                onclick: move |_| settings.write().longer(),
                "\u{2261}+"
            }
        }
    }
}

pub(crate) fn SettingsPopover() -> Element {
    let mut open = use_signal(|| false);

    rsx! {
        PopoverRoot {
            open: open(),
            on_open_change: move |v| open.set(v),
            PopoverTrigger {
                Icon { icon: icon::SETTINGS }
            }
            PopoverContent {
                class: Styles::settings_popover__content.to_string(),
                gap: "0.25rem",
                align: ContentAlign::End,
                div {
                    style: "padding: 0.5rem; display: flex; gap: 0.5rem; flex-direction: column;",
                    onkeydown: move |e| e.stop_propagation(),
                    LineHeightControl {}
                    FontSizeControl {}
                    PageMarginsControl {}
                    MaxLineLengthControl {}
                    FontFamilyPicker {}
                    ThemePicker {}
                }
            }
        }
    }
}
