use crate::web::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Settings {
    pub(crate) theme: Theme,
}

impl Settings {
    pub(crate) fn css_vars(self) -> Vec<(&'static str, String)> {
        self.theme
            .css_vars()
            .into_iter()
            .map(|(name, value)| (name, value.to_string()))
            .collect()
    }

    fn declarations(self) -> String {
        self.css_vars()
            .iter()
            .map(|(name, value)| format!("{name}: {value};"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(crate) fn vars(self) -> String {
        format!(":root {{ {} }}", self.declarations())
    }

    pub(crate) fn user_layer(self) -> String {
        format!(
            "{}\nbody {{ background: var(--USER__backgroundColor) !important; \
                color: var(--USER__textColor) !important; }}",
            self.vars()
        )
    }

    pub(crate) fn inline_styles(self) -> String {
        format!(
            "{} background-color: var(--USER__backgroundColor); color: var(--USER__textColor)",
            self.declarations()
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn the_settings_variable_list_carries_the_whole_palette() {
        for theme in [Theme::Day, Theme::Sepia, Theme::Night] {
            let settings = Settings { theme };
            let vars = settings.css_vars();

            for (name, value) in theme.css_vars() {
                assert!(
                    vars.contains(&(name, value.to_string())),
                    "{theme:?} declares {name}, and the settings list drops it",
                );
            }

            assert_eq!(vars.len(), theme.css_vars().len());
        }
    }

    #[test]
    fn every_theme_sets_both_user_colour_variables() {
        for theme in [Theme::Day, Theme::Sepia, Theme::Night] {
            let settings = Settings { theme };
            let css = settings.vars();

            // The USER layer drives colour through these two, by Readium convention.
            assert!(
                css.contains("--USER__backgroundColor"),
                "{theme:?} has no background"
            );
            assert!(
                css.contains("--USER__textColor"),
                "{theme:?} has no text colour"
            );
            // Step 2 injects this into a document that already has a `<style>`; it has to be
            // a self-contained rule, not a bare declaration list.
            assert!(css.starts_with(":root {"), "{theme:?} is not a :root rule");
        }
    }

    #[test]
    fn the_injected_layer_applies_the_variables_it_declares() {
        for theme in [Theme::Day, Theme::Sepia, Theme::Night] {
            let settings = Settings { theme };
            let css = settings.user_layer();
            let (background, text) = theme.colors();

            assert!(
                css.starts_with(":root {"),
                "{theme:?} does not open with the variable block",
            );
            assert!(css.contains(&format!("--USER__backgroundColor: {background}")));
            assert!(css.contains(&format!("--USER__textColor: {text}")));
            assert!(
                css.contains("var(--USER__backgroundColor)"),
                "{theme:?} declares a background it never applies",
            );
            assert!(
                css.contains("var(--USER__textColor)"),
                "{theme:?} declares a text colour it never applies",
            );
        }
    }

    #[test]
    fn the_pushed_vars_and_the_injected_layer_name_the_same_variables() {
        for theme in [Theme::Day, Theme::Sepia, Theme::Night] {
            let settings = Settings { theme };
            let layer = settings.user_layer();

            // Nothing pushed that the served layer never declares or never applies …
            for (name, value) in settings.css_vars() {
                assert!(
                    layer.contains(&format!("{name}: {value};")),
                    "{theme:?} pushes {name}, which the injected layer never declares",
                );
                assert!(
                    layer.contains(&format!("var({name})")),
                    "{theme:?} declares {name} and no rule reads it",
                );
            }

            // … and nothing read that no message will ever set.
            for reference in layer.split("var(").skip(1) {
                let name = reference.split(')').next().expect("var( … ) closes");
                assert!(
                    settings.css_vars().iter().any(|(pushed, _)| *pushed == name),
                    "the layer reads {name}, which the message never sets — \
                     that variable would only ever update on a chapter turn",
                );
            }
        }
    }

    #[test]
    fn the_three_themes_are_actually_different() {
        let day = Settings { theme: Theme::Day };
        let sepia = Settings {
            theme: Theme::Sepia,
        };
        let night = Settings {
            theme: Theme::Night,
        };

        assert_ne!(day.vars(), night.vars());
        assert_ne!(day.vars(), sepia.vars());
        assert_ne!(sepia.vars(), night.vars());
    }
}
