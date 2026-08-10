use dioxus::prelude::*;
use dioxus_primitives::ContentAlign;

use crate::{
    components::popover::{PopoverContent, PopoverRoot, PopoverTrigger},
    ui::{font::FontFamilyPicker, theme::ThemePicker},
    web::settings::{
        Settings, FONT_SIZE_MAX, FONT_SIZE_MIN, LINE_HEIGHT_MAX, LINE_HEIGHT_MIN,
        MAX_LINE_LENGTH_MAX, MAX_LINE_LENGTH_MIN, PAGE_MARGINS_MAX, PAGE_MARGINS_MIN,
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
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    width: "24",
                    height: "24",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    class: "icon icon-tabler icons-tabler-outline icon-tabler-settings",
                    path {
                        stroke: "none",
                        d: "M0 0h24v24H0z",
                        fill: "none",
                    }
                    path {
                        d: "M10.325 4.317c.426 -1.756 2.924 -1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543 -.94 3.31 .826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.756 .426 1.756 2.924 0 3.35a1.724 1.724 0 0 0 -1.066 2.573c.94 1.543 -.826 3.31 -2.37 2.37a1.724 1.724 0 0 0 -2.572 1.065c-.426 1.756 -2.924 1.756 -3.35 0a1.724 1.724 0 0 0 -2.573 -1.066c-1.543 .94 -3.31 -.826 -2.37 -2.37a1.724 1.724 0 0 0 -1.065 -2.572c-1.756 -.426 -1.756 -2.924 0 -3.35a1.724 1.724 0 0 0 1.066 -2.573c-.94 -1.543 .826 -3.31 2.37 -2.37c1 .608 2.296 .07 2.572 -1.065",
                    }
                    path {
                        d: "M9 12a3 3 0 1 0 6 0a3 3 0 0 0 -6 0",
                    }
                }
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
