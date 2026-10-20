use std::rc::Rc;

use dioxus::prelude::*;

use crate::{
    ai::{prompt::DEFAULT_PROMPT_TEMPLATE, Provider},
    library::Library,
    secrets::{api_key, SecretStore},
    settings::{
        choice::Choice, Settings, FONT_SIZE_MAX, FONT_SIZE_MIN, LINE_HEIGHT_MAX, LINE_HEIGHT_MIN,
        MAX_LINE_LENGTH_MAX, MAX_LINE_LENGTH_MIN, PAGE_MARGINS_MAX, PAGE_MARGINS_MIN,
    },
    ui::{
        components::icon::{self, Icon},
        drawer::Drawer,
        OrLog,
    },
};

#[css_module("/src/ui/settings.css")]
pub(crate) struct Styles;

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
                onpointerdown: move |e| e.stop_propagation(),
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

fn use_key_set(provider: Provider) -> Signal<bool> {
    let store = use_context::<Option<Rc<dyn SecretStore>>>();
    use_signal(|| {
        store
            .as_deref()
            .and_then(|store| {
                api_key::is_set(store, provider)
                    .or_log(&format!("read the {} API key", provider.label()))
            })
            .unwrap_or(false)
    })
}

#[component]
fn ProviderSettings(provider: Provider, note: Option<&'static str>) -> Element {
    let mut key_set = use_key_set(provider);

    rsx! {
        h3 { class: "{Styles::settings_group_title}", {provider.label()} }
        div {
            class: "{Styles::settings_group}",
            ApiKeyControl {
                provider,
                key_set: key_set(),
                on_change: move |key: Option<String>| key_set.set(key.is_some()),
            }
        }
        if let Some(note) = note {
            p { class: "{Styles::settings_group_note}", {note} }
        }
    }
}

#[component]
pub(crate) fn GeminiSettings() -> Element {
    rsx! {
        ProviderSettings { provider: Provider::Gemini }
    }
}

#[component]
pub(crate) fn OpenAiSettings() -> Element {
    rsx! {
        ProviderSettings {
            provider: Provider::OpenAi,
            note: "OpenAI keeps API requests for 30 days by default.",
        }
    }
}

const PLACEHOLDER_HINT: &str = "Placeholders: {title} {author} {chapter} {selection}";

#[component]
fn PromptTemplateEditor(
    label: &'static str,
    status: &'static str,
    custom: bool,
    reset_label: &'static str,
    text: String,
    on_save: EventHandler<String>,
    on_reset: EventHandler,
) -> Element {
    let mut textarea_generation = use_signal(|| 0_u32);

    rsx! {
        div {
            class: "{Styles::settings_group}",
            SettingRow {
                label: status,
                button {
                    class: "{Styles::pill_button}",
                    disabled: !custom,
                    onclick: move |_| {
                        on_reset.call(());
                        textarea_generation += 1;
                    },
                    {reset_label}
                }
            }
            textarea {
                key: "{textarea_generation}",
                class: "{Styles::prompt_template}",
                aria_label: label,
                spellcheck: "false",
                value: "{text}",
                onchange: move |event| {
                    on_save.call(event.value());
                    textarea_generation += 1;
                },
            }
        }
        p { class: "{Styles::settings_group_note}", {PLACEHOLDER_HINT} }
    }
}

#[component]
pub(crate) fn GlobalPromptTemplateSettings() -> Element {
    let library = use_context::<Rc<Library>>();
    let mut global = use_signal(|| read_global_prompt_template(&library));
    let custom = global.read().is_some();
    let status = if custom { "Custom" } else { "Default" };
    let text = global().unwrap_or_else(|| DEFAULT_PROMPT_TEMPLATE.to_string());

    rsx! {
        h3 { class: "{Styles::settings_group_title}", "Prompt template" }
        PromptTemplateEditor {
            label: "Global prompt template",
            status,
            custom,
            reset_label: "Reset to default",
            text,
            on_save: {
                let library = library.clone();
                move |template: String| {
                    library
                        .set_global_prompt_template(&template)
                        .or_log("save the global prompt template");
                    global.set(read_global_prompt_template(&library));
                }
            },
            on_reset: move |_| {
                library
                    .clear_global_prompt_template()
                    .or_log("reset the global prompt template");
                global.set(read_global_prompt_template(&library));
            },
        }
    }
}

fn read_global_prompt_template(library: &Library) -> Option<String> {
    library
        .global_prompt_template()
        .or_log("read the global prompt template")
        .flatten()
}

pub(crate) fn read_effective_prompt_template(library: &Library, book_id: i64) -> String {
    library
        .effective_prompt_template(book_id)
        .or_log("read the effective prompt template")
        .unwrap_or_else(|| DEFAULT_PROMPT_TEMPLATE.to_string())
}

#[component]
fn BookPromptTemplateSettings(book_id: ReadSignal<i64>, open: ReadSignal<bool>) -> Element {
    let library = use_context::<Rc<Library>>();
    let mut book_template_writes = use_signal(|| 0_u32);
    let stored = use_memo({
        let library = library.clone();
        move || {
            let _reread_on_open = open();
            let _reread_after_write = book_template_writes();
            StoredBookPromptTemplate::read(&library, book_id())
        }
    });
    let StoredBookPromptTemplate { custom, text } = stored();
    let status = if custom {
        "Custom template"
    } else {
        "Following global"
    };

    rsx! {
        h3 { class: "{Styles::settings_group_title}", "This book" }
        PromptTemplateEditor {
            label: "Book prompt template",
            status,
            custom,
            reset_label: "Use global",
            text,
            on_save: {
                let library = library.clone();
                move |template: String| {
                    library
                        .set_book_prompt_template(book_id(), &template)
                        .or_log("save the book prompt template");
                    book_template_writes += 1;
                }
            },
            on_reset: move |_| {
                library
                    .clear_book_prompt_template(book_id())
                    .or_log("use the global prompt template for the book");
                book_template_writes += 1;
            },
        }
    }
}

#[derive(Clone, PartialEq)]
struct StoredBookPromptTemplate {
    custom: bool,
    text: String,
}

impl StoredBookPromptTemplate {
    fn read(library: &Library, book_id: i64) -> Self {
        let custom = library
            .book_prompt_template(book_id)
            .or_log("read the book prompt template")
            .flatten()
            .is_some();
        let text = read_effective_prompt_template(library, book_id);
        Self { custom, text }
    }
}

#[component]
pub(crate) fn ReadingSettingsDrawer(book_id: i64) -> Element {
    let mut open = use_signal(|| false);

    rsx! {
        button {
            class: "icon-button",
            aria_label: "Reading settings",
            onclick: move |_| open.set(true),
            Icon { icon: icon::SETTINGS }
        }
        Drawer {
            open,
            label: "Reading settings",
            div {
                class: "{Styles::reading_settings}",
                ReaderThemeControls {}
                BookPromptTemplateSettings { book_id, open }
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
