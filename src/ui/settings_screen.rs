use dioxus::prelude::*;

use crate::ui::{
    components::icon::{self, Icon},
    settings::{GeminiSettings, ReaderThemeControls},
};

#[css_module("/src/ui/settings_screen.css")]
struct Styles;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SettingsSection {
    #[default]
    ReaderTheme,
    Ai,
}

impl SettingsSection {
    pub(crate) const ALL: [SettingsSection; 2] =
        [SettingsSection::ReaderTheme, SettingsSection::Ai];

    pub(crate) fn label(self) -> &'static str {
        match self {
            SettingsSection::ReaderTheme => "Reader theme",
            SettingsSection::Ai => "AI",
        }
    }
}

#[component]
pub(crate) fn SettingsScreen(mut open: Signal<bool>) -> Element {
    let mut chosen = use_signal(|| None::<SettingsSection>);
    let chosen_now = chosen();
    let current = chosen_now.unwrap_or_default();

    rsx! {
        div {
            class: "{Styles::settings_screen}",
            role: "dialog",
            aria_label: "Settings",
            "data-section-chosen": if chosen_now.is_some() { "true" },
            header {
                class: "{Styles::settings_screen__header}",
                button {
                    class: "icon-button {Styles::settings_screen__back}",
                    aria_label: "Back to settings",
                    onclick: move |_| chosen.set(None),
                    Icon { icon: icon::CHEVRON_LEFT }
                }
                h1 { class: "{Styles::settings_screen__title}", "Settings" }
                button {
                    class: "icon-button",
                    aria_label: "Close settings",
                    onclick: move |_| open.set(false),
                    Icon { icon: icon::CLOSE }
                }
            }
            div {
                class: "{Styles::settings_screen__body}",
                nav {
                    class: "{Styles::settings_screen__sidebar}",
                    aria_label: "Settings sections",
                    for item in SettingsSection::ALL {
                        button {
                            class: "{Styles::settings_screen__section_button}",
                            aria_current: if item == current { "page" },
                            onclick: move |_| chosen.set(Some(item)),
                            span { {item.label()} }
                            Icon { icon: icon::CHEVRON_RIGHT }
                        }
                    }
                }
                section {
                    class: "{Styles::settings_screen__section}",
                    div {
                        class: "{Styles::settings_screen__content}",
                        h2 { class: "{Styles::settings_screen__heading}", {current.label()} }
                        match current {
                            SettingsSection::ReaderTheme => rsx! { ReaderThemeControls {} },
                            SettingsSection::Ai => rsx! { GeminiSettings {} },
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn the_sections_are_reader_theme_then_ai() {
        assert_eq!(
            SettingsSection::ALL.map(SettingsSection::label),
            ["Reader theme", "AI"]
        );
    }

    #[test]
    fn settings_open_on_the_reader_theme() {
        assert_eq!(SettingsSection::default(), SettingsSection::ReaderTheme);
    }
}
