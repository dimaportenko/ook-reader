use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_primitives::ContentAlign;

use crate::{
    ai::{gemini::Gemini, opencode, Provider},
    secrets::{api_key, SecretStore},
    settings::{
        choice::Choice, Settings, FONT_SIZE_MAX, FONT_SIZE_MIN, LINE_HEIGHT_MAX,
        LINE_HEIGHT_MIN, MAX_LINE_LENGTH_MAX, MAX_LINE_LENGTH_MIN, PAGE_MARGINS_MAX,
        PAGE_MARGINS_MIN,
    },
    ui::{
        components::{
            icon::{self, Icon},
            popover::{PopoverContent, PopoverRoot, PopoverTrigger},
        },
        OrLog,
    },
};

#[css_module("/src/ui/settings.css")]
struct Styles;

#[component]
fn SettingRow(label: &'static str, children: Element) -> Element {
    rsx! {
        div {
            class: "{Styles::settings_row}",
            span { class: "{Styles::settings_row__label}", {label} }
            div { class: "{Styles::settings_row__control}", {children} }
        }
    }
}

#[component]
fn Stepper(
    label: &'static str,
    value: String,
    can_decrease: bool,
    can_increase: bool,
    on_decrease: EventHandler,
    on_increase: EventHandler,
) -> Element {
    rsx! {
        SettingRow {
            label,
            div {
                class: "{Styles::stepper}",
                role: "group",
                aria_label: label,
                button {
                    class: "{Styles::stepper__button}",
                    aria_label: "Decrease {label}",
                    disabled: !can_decrease,
                    onclick: move |_| on_decrease.call(()),
                    Icon { icon: icon::MINUS }
                }
                output { class: "{Styles::stepper__value}", "{value}" }
                button {
                    class: "{Styles::stepper__button}",
                    aria_label: "Increase {label}",
                    disabled: !can_increase,
                    onclick: move |_| on_increase.call(()),
                    Icon { icon: icon::ADD }
                }
            }
        }
    }
}

#[component]
fn ChoiceRow<T: Choice>(label: &'static str, selected: T, on_pick: EventHandler<T>) -> Element {
    rsx! {
        SettingRow {
            label,
            select {
                class: "{Styles::pill_button} {Styles::choice}",
                aria_label: label,
                onchange: move |event| on_pick.call(T::from_slug(&event.value())),
                for choice in T::all().iter().copied() {
                    option {
                        key: "{choice.slug()}",
                        value: choice.slug(),
                        selected: choice == selected,
                        {choice.label()}
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
fn ApiKeyControl(
    provider: Provider,
    key_set: bool,
    on_change: EventHandler<Option<String>>,
) -> Element {
    let store = use_context::<Option<Rc<dyn SecretStore>>>();
    let mut draft = use_signal(String::new);
    let status = KeyStatus::of(store.is_some(), key_set);
    let name = provider.label();

    rsx! {
        SettingRow {
            label: "API key",
            span {
                class: "{Styles::key_status}",
                "data-set": if key_set { "true" },
                {status.label()}
            }
        }
        if let Some(store) = store {
            div {
                class: "{Styles::key_form}",
                input {
                    class: "{Styles::key_form__input}",
                    r#type: "password",
                    aria_label: "{name} API key",
                    placeholder: if key_set { "Replace key" } else { "Paste your key" },
                    autocomplete: "off",
                    spellcheck: "false",
                    value: "{draft}",
                    oninput: move |event| draft.set(event.data.value()),
                }
                button {
                    class: "{Styles::pill_button}",
                    disabled: draft.read().trim().is_empty(),
                    onclick: {
                        let store = store.clone();
                        move |_| {
                            let saved = api_key::save(store.as_ref(), provider, &draft.read());
                            if let Some(key) = saved.or_log(&format!("save the {name} API key")) {
                                on_change.call(Some(key));
                                draft.set(String::new());
                            }
                        }
                    },
                    "Save"
                }
                if key_set {
                    button {
                        class: "{Styles::pill_button} {Styles::pill_button__danger}",
                        onclick: {
                            let store = store.clone();
                            move |_| {
                                if api_key::forget(store.as_ref(), provider)
                                    .or_log(&format!("forget the {name} API key"))
                                    .is_some()
                                {
                                    on_change.call(None);
                                }
                            }
                        },
                        "Forget"
                    }
                }
            }
        }
    }
}

#[component]
pub(crate) fn ReaderThemeControls() -> Element {
    let mut settings = use_context::<Signal<Settings>>();
    let current = settings();

    rsx! {
        div {
            class: "{Styles::settings_group}",
            ChoiceRow {
                label: "Theme",
                selected: current.theme,
                on_pick: move |theme| settings.write().theme = theme,
            }
            ChoiceRow {
                label: "Font",
                selected: current.font_family,
                on_pick: move |font_family| settings.write().font_family = font_family,
            }
            Stepper {
                label: "Font size",
                value: format!("{}%", current.font_size),
                can_decrease: current.font_size > FONT_SIZE_MIN,
                can_increase: current.font_size < FONT_SIZE_MAX,
                on_decrease: move |_| settings.write().zoom_out(),
                on_increase: move |_| settings.write().zoom_in(),
            }
            Stepper {
                label: "Line height",
                value: current.line_height_css(),
                can_decrease: current.line_height > LINE_HEIGHT_MIN,
                can_increase: current.line_height < LINE_HEIGHT_MAX,
                on_decrease: move |_| settings.write().tighter(),
                on_increase: move |_| settings.write().looser(),
            }
            Stepper {
                label: "Margins",
                value: current.page_margins_css(),
                can_decrease: current.page_margins > PAGE_MARGINS_MIN,
                can_increase: current.page_margins < PAGE_MARGINS_MAX,
                on_decrease: move |_| settings.write().narrower(),
                on_increase: move |_| settings.write().wider(),
            }
            Stepper {
                label: "Line length",
                value: current.max_line_length.to_string(),
                can_decrease: current.max_line_length > MAX_LINE_LENGTH_MIN,
                can_increase: current.max_line_length < MAX_LINE_LENGTH_MAX,
                on_decrease: move |_| settings.write().shorter(),
                on_increase: move |_| settings.write().longer(),
            }
        }
    }
}

#[component]
pub(crate) fn GeminiSettings() -> Element {
    let mut settings = use_context::<Signal<Settings>>();
    let mut gemini = use_context::<Signal<Option<Gemini>>>();

    rsx! {
        h3 { class: "{Styles::settings_group_title}", "Gemini" }
        div {
            class: "{Styles::settings_group}",
            ApiKeyControl {
                provider: Provider::Gemini,
                key_set: gemini.read().is_some(),
                on_change: move |key: Option<String>| {
                    gemini.set(key.map(|key| Gemini::new(key, settings().ai_model.api_name())));
                },
            }
            ChoiceRow {
                label: "Model",
                selected: settings().ai_model,
                on_pick: move |ai_model| settings.write().ai_model = ai_model,
            }
        }
    }
}

#[component]
pub(crate) fn OpenCodeZenSettings() -> Element {
    let store = use_context::<Option<Rc<dyn SecretStore>>>();
    let mut key_set = use_signal(|| {
        store
            .as_deref()
            .and_then(|store| {
                api_key::is_set(store, Provider::OpenCodeZen)
                    .or_log("read the OpenCode Zen API key")
            })
            .unwrap_or(false)
    });

    rsx! {
        h3 { class: "{Styles::settings_group_title}", "OpenCode Zen" }
        div {
            class: "{Styles::settings_group}",
            ApiKeyControl {
                provider: Provider::OpenCodeZen,
                key_set: key_set(),
                on_change: move |key: Option<String>| key_set.set(key.is_some()),
            }
            if key_set() {
                ZenCatalog {}
            }
        }
    }
}

#[component]
fn ZenCatalog() -> Element {
    let catalog = use_resource(opencode::models);

    rsx! {
        match &*catalog.read() {
            None => rsx! {
                SettingRow {
                    label: "Models",
                    span { class: "{Styles::key_status}", "Loading…" }
                }
            },
            Some(Err(error)) => rsx! {
                p { class: "{Styles::catalog_error}", "Could not load the model list: {error}" }
            },
            Some(Ok(ids)) => rsx! {
                for id in ids {
                    div { key: "{id}", class: "{Styles::settings_row}", {id.as_str()} }
                }
            },
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
                ReaderThemeControls {}
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
