use crate::web::theme::Theme;

#[cfg(test)]
use crate::web::assets::INJECTED_ASSETS;

pub(crate) const FONT_SIZE_MIN: u16 = 75;
pub(crate) const FONT_SIZE_MAX: u16 = 250;
pub(crate) const FONT_SIZE_STEP: u16 = 25;

pub(crate) const LINE_HEIGHT_MIN: u16 = 100;
pub(crate) const LINE_HEIGHT_MAX: u16 = 200;
pub(crate) const LINE_HEIGHT_STEP: u16 = 10;

pub(crate) const PAGE_MARGINS_MIN: u16 = 50;
pub(crate) const PAGE_MARGINS_MAX: u16 = 200;
pub(crate) const PAGE_MARGINS_STEP: u16 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Settings {
    pub(crate) theme: Theme,
    pub(crate) font_size: u16,
    pub(crate) line_height: u16,
    pub(crate) page_margins: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::default(),
            font_size: 100,
            line_height: 140,
            page_margins: 100,
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

    pub(crate) fn looser(&mut self) {
        self.line_height = self
            .line_height
            .saturating_add(LINE_HEIGHT_STEP)
            .min(LINE_HEIGHT_MAX);
    }

    pub(crate) fn tighter(&mut self) {
        self.line_height = self
            .line_height
            .saturating_sub(LINE_HEIGHT_STEP)
            .max(LINE_HEIGHT_MIN);
    }

    pub(crate) fn wider(&mut self) {
        self.page_margins = self
            .page_margins
            .saturating_add(PAGE_MARGINS_STEP)
            .min(PAGE_MARGINS_MAX);
    }

    pub(crate) fn narrower(&mut self) {
        self.page_margins = self
            .page_margins
            .saturating_sub(PAGE_MARGINS_STEP)
            .max(PAGE_MARGINS_MIN);
    }

    pub(crate) fn css_vars(self) -> Vec<(&'static str, String)> {
        let mut vars = self
            .theme
            .css_vars()
            .into_iter()
            .map(|(name, value)| (name, value.to_string()))
            .collect::<Vec<_>>();

        vars.push(("--USER__fontSize", format!("{}%", self.font_size)));
        vars.push(("--USER__lineHeight", self.line_height_css()));
        vars.push(("--USER__pageMargins", self.page_margins_css()));
        vars
    }

    pub(crate) fn line_height_css(self) -> String {
        format!("{}.{:02}", self.line_height / 100, self.line_height % 100)
    }

    pub(crate) fn page_margins_css(self) -> String {
        format!("{}.{:02}", self.page_margins / 100, self.page_margins % 100)
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
                color: var(--USER__textColor) !important; }} \
                \nbody, body * {{ line-height: var(--USER__lineHeight) !important; }}",
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
                theme.css_vars().len() + 3,
                "the palette plus --USER__fontSize, --USER__lineHeight and \
                 --USER__pageMargins — bump this when a setting is added",
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
            // A pushed variable must be read by *something the document gets*, which is no
            // longer only the layer: the geometry rules live in pagination.css, served
            // ahead of the layer and reading the value the layer sets.
            let readers = format!("{layer}{INJECTED_ASSETS}");

            // Nothing pushed that the served layer never declares …
            for (name, value) in settings.css_vars() {
                assert!(
                    layer.contains(&format!("{name}: {value};")),
                    "{theme:?} pushes {name}, which the injected layer never declares",
                );
                assert!(
                    readers.contains(&format!("var({name})"))
                        || readers.contains(&format!("var({name},")),
                    "{theme:?} declares {name} and no rule reads it",
                );
            }

            // … and nothing the *layer* reads that no message will ever set. This half
            // stays narrow on purpose: INJECTED_ASSETS reads --ook-page, --ook-column and
            // --RS__pageGutter, which are internal and not settings.
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

    #[test]
    fn the_line_height_reaches_the_layer_unitless() {
        let settings = Settings {
            line_height: 140,
            ..Settings::default()
        };

        assert!(settings
            .css_vars()
            .contains(&("--USER__lineHeight", "1.40".to_string())));

        let layer = settings.user_layer();

        assert!(
            layer.contains("--USER__lineHeight: 1.40;"),
            "the chosen leading never reached the :root block",
        );
        assert!(
            layer.contains("line-height: var(--USER__lineHeight)"),
            "the layer declares a leading it never applies — the number would move \
             and the lines would not",
        );
    }

    #[test]
    fn a_line_height_below_a_tenth_keeps_its_leading_zero() {
        // 105 hundredths is 1.05, not 1.5. `{}.{}` prints the latter, and the gap
        // between "a hair looser" and "half again as loose" is invisible in the
        // source and obvious on screen.
        let settings = Settings {
            line_height: 105,
            ..Settings::default()
        };

        assert!(settings
            .css_vars()
            .contains(&("--USER__lineHeight", "1.05".to_string())));
    }

    #[test]
    fn the_line_height_rule_reaches_the_elements_the_book_styles() {
        // A rule on `body` alone only supplies an *inherited* value, and inheritance
        // fills in what nothing else declares. A book that says `p { line-height: 1.2 }`
        // has declared it, so the paragraphs — the only text you actually read — would
        // ignore the setting entirely.
        assert!(Settings::default().user_layer().contains("body *"));
    }

    #[test]
    fn the_background_rule_stops_at_the_body() {
        for rule in Settings::default().user_layer().split('}') {
            let Some((selector, declarations)) = rule.split_once('{') else {
                continue;
            };

            assert!(
                !selector.contains('*') || !declarations.contains("background"),
                "`{}` paints an !important background on every element — a book's own \
                 backgrounds lose, and the shorthand drops their images with them. \
                 The leading is the only thing that needs the descendant selector.",
                selector.trim(),
            );
        }
    }

    #[test]
    fn the_page_margins_reach_the_layer_as_a_bare_factor() {
        let settings = Settings {
            page_margins: 150,
            ..Settings::default()
        };

        assert!(settings
            .css_vars()
            .contains(&("--USER__pageMargins", "1.50".to_string())));

        // No unit. The value is a multiplicand inside `calc(2 * 24px * m)`; give it a
        // unit and that product is an area, the calc is invalid, and the declaration
        // falls back to its initial value — which for `padding` is 0, i.e. no margin
        // at all on the setting that exists to add one.
        assert!(
            settings.vars().contains("--USER__pageMargins: 1.50;"),
            "the chosen margin never reached the :root block",
        );
    }

    #[test]
    fn the_page_margins_step_and_clamp() {
        let mut settings = Settings {
            page_margins: 150,
            ..Settings::default()
        };

        settings.narrower();
        assert_eq!(settings.page_margins, 150 - PAGE_MARGINS_STEP);
        settings.wider();
        assert_eq!(settings.page_margins, 150);

        for _ in 0..20 {
            settings.narrower();
        }
        assert_eq!(settings.page_margins, PAGE_MARGINS_MIN);

        for _ in 0..20 {
            settings.wider();
        }
        assert_eq!(settings.page_margins, PAGE_MARGINS_MAX);
    }

    #[test]
    fn the_line_height_steps_and_clamps() {
        let mut settings = Settings {
            line_height: 150,
            ..Settings::default()
        };

        settings.tighter();
        assert_eq!(settings.line_height, 150 - LINE_HEIGHT_STEP);
        settings.looser();
        assert_eq!(settings.line_height, 150);

        for _ in 0..20 {
            settings.tighter();
        }
        assert_eq!(settings.line_height, LINE_HEIGHT_MIN);

        for _ in 0..20 {
            settings.looser();
        }
        assert_eq!(settings.line_height, LINE_HEIGHT_MAX);
    }
}
