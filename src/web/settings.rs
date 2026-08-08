use crate::web::theme::Theme;

pub(crate) const FONT_SIZE_MIN: u16 = 75;
pub(crate) const FONT_SIZE_MAX: u16 = 250;
pub(crate) const FONT_SIZE_STEP: u16 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Settings {
    pub(crate) theme: Theme,
    pub(crate) font_size: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::default(),
            font_size: 100,
        }
    }
}

impl Settings {
    pub(crate) fn zoom_in(&mut self) {
        self.font_size = self
            .font_size
            .saturating_add(FONT_SIZE_STEP)
            .min(FONT_SIZE_MAX);
    }

    pub(crate) fn zoom_out(&mut self) {
        self.font_size = self
            .font_size
            .saturating_sub(FONT_SIZE_STEP)
            .max(FONT_SIZE_MIN);
    }

    pub(crate) fn css_vars(self) -> Vec<(&'static str, String)> {
        let mut vars = self
            .theme
            .css_vars()
            .into_iter()
            .map(|(name, value)| (name, value.to_string()))
            .collect::<Vec<_>>();

        vars.push(("--USER__fontSize", format!("{}%", self.font_size)));
        vars
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
            "{}\nhtml {{ font-size: var(--USER__fontSize) !important; }} \
                \nbody {{ background: var(--USER__backgroundColor) !important; \
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
            let settings = Settings {
                theme,
                ..Settings::default()
            };
            let vars = settings.css_vars();

            for (name, value) in theme.css_vars() {
                assert!(
                    vars.contains(&(name, value.to_string())),
                    "{theme:?} declares {name}, and the settings list drops it",
                );
            }

            assert_eq!(
                vars.len(),
                theme.css_vars().len() + 1,
                "the palette plus --USER__fontSize — bump this when a setting is added",
            );
        }
    }

    #[test]
    fn every_theme_sets_both_user_colour_variables() {
        for theme in [Theme::Day, Theme::Sepia, Theme::Night] {
            let settings = Settings {
                theme,
                ..Settings::default()
            };
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
            let settings = Settings {
                theme,
                ..Settings::default()
            };
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
            let settings = Settings {
                theme,
                ..Settings::default()
            };
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
                    settings
                        .css_vars()
                        .iter()
                        .any(|(pushed, _)| *pushed == name),
                    "the layer reads {name}, which the message never sets — \
                     that variable would only ever update on a chapter turn",
                );
            }
        }
    }

    #[test]
    fn the_three_themes_are_actually_different() {
        let day = Settings {
            theme: Theme::Day,
            ..Settings::default()
        };
        let sepia = Settings {
            theme: Theme::Sepia,
            ..Settings::default()
        };
        let night = Settings {
            theme: Theme::Night,
            ..Settings::default()
        };

        assert_ne!(day.vars(), night.vars());
        assert_ne!(day.vars(), sepia.vars());
        assert_ne!(sepia.vars(), night.vars());
    }

    #[test]
    fn the_default_font_size_is_100_percent() {
        // Not a style preference: a derived `Default` gives `0`, which serves
        // `font-size: 0%` to every caller of `Settings::default()`.
        assert_eq!(Settings::default().font_size, 100);
    }

    #[test]
    fn the_font_size_steps_and_clamps() {
        let mut settings = Settings {
            font_size: 150,
            ..Settings::default()
        };

        settings.zoom_out();
        assert_eq!(settings.font_size, 150 - FONT_SIZE_STEP);
        settings.zoom_in();
        assert_eq!(settings.font_size, 150);

        for _ in 0..20 {
            settings.zoom_out();
        }
        assert_eq!(
            settings.font_size, FONT_SIZE_MIN,
            "zooming out past the floor must clamp, not underflow",
        );

        for _ in 0..20 {
            settings.zoom_in();
        }
        assert_eq!(
            settings.font_size, FONT_SIZE_MAX,
            "zooming in past the ceiling must clamp, not overflow",
        );
    }

    #[test]
    fn the_font_size_reaches_the_layer_as_a_percentage() {
        let settings = Settings {
            font_size: 125,
            ..Settings::default()
        };

        assert!(settings
            .css_vars()
            .contains(&("--USER__fontSize", "125%".to_string())));

        let layer = settings.user_layer();

        assert!(
            layer.contains("--USER__fontSize: 125%;"),
            "the chosen size never reached the :root block",
        );
        assert!(
            layer.contains("font-size: var(--USER__fontSize)"),
            "the layer declares a size it never applies — the number would move \
             and the text would not",
        );
    }
}
