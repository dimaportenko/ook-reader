use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_primitives::ContentAlign;

use crate::{
    ai::gemini::Gemini,
    save_gemini_key,
    secrets::SecretStore,
    settings::{
        ai_model::AiModel, Settings, FONT_SIZE_MAX, FONT_SIZE_MIN, LINE_HEIGHT_MAX,
        LINE_HEIGHT_MIN, MAX_LINE_LENGTH_MAX, MAX_LINE_LENGTH_MIN, PAGE_MARGINS_MAX,
        PAGE_MARGINS_MIN,
    },
    ui::{
        components::{
            icon::{self, Icon},
            popover::{PopoverContent, PopoverRoot, PopoverTrigger},
        },
        font::FontFamilyPicker,
        theme::ThemePicker,
        OrLog,
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

#[component]
pub(crate) fn AiModelPicker() -> Element {
    let mut settings = use_context::<Signal<Settings>>();

    rsx! {
        label {
            "AI model"
            select {
                onchange: move |event| {
                    settings.write().ai_model = AiModel::from_slug(&event.data.value());
                },
                for model in AiModel::ALL {
                    option {
                        key: "{model.slug()}",
                        value: model.slug(),
                        selected: model == settings().ai_model,
                        {model.label()}
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyStatus {
    Unavailable,
    NotSet,
    Set,
}

impl KeyStatus {
    fn of(store_open: bool, provider_ready: bool) -> Self {
        match (store_open, provider_ready) {
            (false, _) => KeyStatus::Unavailable,
            (true, false) => KeyStatus::NotSet,
            (true, true) => KeyStatus::Set,
        }
    }

    fn label(self) -> &'static str {
        match self {
            KeyStatus::Unavailable => "Secret store unavailable",
            KeyStatus::NotSet => "Not set",
            KeyStatus::Set => "Key set",
        }
    }
}

#[component]
pub(crate) fn ApiKeyControl() -> Element {
    let store = use_context::<Option<Rc<dyn SecretStore>>>();
    let mut provider = use_context::<Signal<Option<Gemini>>>();
    let settings = use_context::<Signal<Settings>>();
    let mut draft = use_signal(String::new);
    let status = KeyStatus::of(store.is_some(), provider.read().is_some());

    rsx! {
        div {
            "Gemini API key"
            span {
                style: "padding: 0 0.5rem",
                {status.label()}
            }
            if let Some(store) = store {
                input {
                    r#type: "password",
                    value: "{draft}",
                    oninput: move |event| draft.set(event.data.value()),
                }
                button {
                    disabled: draft.read().trim().is_empty(),
                    onclick: move |_| {
                        let saved = save_gemini_key(store.as_ref(), &draft.read(), settings().ai_model);
                        if let Some(gemini) = saved.or_log("save the Gemini API key") {
                            provider.set(Some(gemini));
                            draft.set(String::new());
                        }
                    },
                    "Save"
                }
            }
        }
    }
}

pub(crate) fn SettingsPopover() -> Element {
    rsx! {
        PopoverRoot {
            PopoverTrigger {
                aria_label: "Reading settings",
                Icon { icon: icon::SETTINGS }
            }
            PopoverContent {
                class: Styles::settings_popover__content.to_string(),
                gap: "0.25rem",
                align: ContentAlign::End,
                div {
                    style: "padding: 0.5rem; display: flex; gap: 0.5rem; flex-direction: column;",
                    LineHeightControl {}
                    FontSizeControl {}
                    PageMarginsControl {}
                    MaxLineLengthControl {}
                    FontFamilyPicker {}
                    ThemePicker {}
                    AiModelPicker {}
                    ApiKeyControl {}
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn key_status_needs_an_open_store_before_it_can_report_a_key() {
        assert_eq!(KeyStatus::of(false, false), KeyStatus::Unavailable);
        assert_eq!(KeyStatus::of(false, true), KeyStatus::Unavailable);
        assert_eq!(KeyStatus::of(true, false), KeyStatus::NotSet);
        assert_eq!(KeyStatus::of(true, true), KeyStatus::Set);
    }

    #[test]
    fn each_key_status_has_a_reader_facing_label() {
        assert_eq!(KeyStatus::Unavailable.label(), "Secret store unavailable");
        assert_eq!(KeyStatus::NotSet.label(), "Not set");
        assert_eq!(KeyStatus::Set.label(), "Key set");
    }
}
