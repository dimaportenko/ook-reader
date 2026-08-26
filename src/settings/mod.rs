pub mod font;
pub mod theme;

use crate::settings::{font::FontFamily, theme::Theme};

use crate::web::assets::USER_LAYER_RULES;

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

pub(crate) const MAX_LINE_LENGTH_MIN: u16 = 45;
pub(crate) const MAX_LINE_LENGTH_MAX: u16 = 100;
pub(crate) const MAX_LINE_LENGTH_STEP: u16 = 5;

fn hundredths(value: u16) -> String {
    format!("{}.{:02}", value / 100, value % 100)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Settings {
    pub(crate) theme: Theme,
    pub(crate) font_family: FontFamily,
    pub(crate) font_size: u16,
    pub(crate) line_height: u16,
    pub(crate) page_margins: u16,
    pub(crate) max_line_length: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::default(),
            font_family: FontFamily::default(),
            font_size: 100,
            line_height: 140,
            page_margins: 100,
            max_line_length: 70,
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

    pub(crate) fn longer(&mut self) {
        self.max_line_length = self
            .max_line_length
            .saturating_add(MAX_LINE_LENGTH_STEP)
            .min(MAX_LINE_LENGTH_MAX);
    }

    pub(crate) fn shorter(&mut self) {
        self.max_line_length = self
            .max_line_length
            .saturating_sub(MAX_LINE_LENGTH_STEP)
            .max(MAX_LINE_LENGTH_MIN);
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
        vars.push((
            "--USER__maxLineLength",
            format!("{}ch", self.max_line_length),
        ));
        vars.push(("--USER__fontFamily", self.font_family.stack().to_string()));
        vars
    }

    pub(crate) fn line_height_css(self) -> String {
        hundredths(self.line_height)
    }

    pub(crate) fn page_margins_css(self) -> String {
        hundredths(self.page_margins)
    }

    fn declarations(self) -> String {
        self.css_vars()
            .iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(name, value)| format!("{name}: {value};"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(crate) fn bootstrap_js(self) -> String {
        let stack = self.font_family.stack();
        if stack.is_empty() {
            return String::new();
        }

        format!("document.documentElement.style.setProperty(\"--USER__fontFamily\", \"{stack}\");")
    }

    pub(crate) fn vars(self) -> String {
        format!(":root {{ {} }}", self.declarations())
    }

    pub(crate) fn user_layer(self) -> String {
        format!("{}\n{USER_LAYER_RULES}", self.vars())
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
                theme.css_vars().len() + 5,
                "the palette plus --USER__fontSize, --USER__lineHeight, \
                 --USER__pageMargins, --USER__maxLineLength and --USER__fontFamily — \
                 bump this when a setting is added",
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
                // … except an *empty* value, whose whole meaning is "no declaration".
                // `--USER__fontFamily: ;` is not a declaration; the parser drops it and
                // the served CSS would carry a statement that is not true.
                if value.is_empty() {
                    assert!(
                        !layer.contains(&format!("{name}:")),
                        "{theme:?} pushes {name} unset and the layer declares it anyway",
                    );
                    continue;
                }

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

    fn size_rule(layer: &str) -> &str {
        layer
            .split('\n')
            .find(|rule| rule.contains("font-size: inherit"))
            .expect(
                "the layer sizes only <html>, so a book whose paragraphs say \
                 `font-size: medium` never moves",
            )
    }

    #[test]
    fn the_size_rule_reaches_the_text_the_publisher_pinned() {
        let layer = Settings::default().user_layer();
        let rule = size_rule(&layer);

        assert!(
            rule.starts_with("body, body *"),
            "`{}` does not reach the elements the book sized itself",
            rule.split('{').next().unwrap_or(rule).trim(),
        );
        assert!(
            rule.contains("!important"),
            "the book's own `font-size` is a later, equally weighted declaration and wins",
        );
    }

    #[test]
    fn the_size_rule_spares_the_elements_whose_size_is_relative() {
        let layer = Settings::default().user_layer();

        for tag in ["h1", "h2", "h3", "h4", "h5", "h6", "sub", "sup", "small"] {
            assert!(
                size_rule(&layer).contains(&format!(":not({tag})")),
                "<{tag}> is sized as a fraction of its parent, so it already tracks the \
                 setting — flattening it to `inherit` costs the hierarchy and buys nothing",
            );
        }
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
    fn the_measure_reaches_the_layer_in_characters() {
        let settings = Settings {
            max_line_length: 66,
            ..Settings::default()
        };

        assert!(settings
            .css_vars()
            .contains(&("--USER__maxLineLength", "66ch".to_string())));
        assert!(
            settings.vars().contains("--USER__maxLineLength: 66ch;"),
            "the chosen measure never reached the :root block",
        );

        // The unit is the whole decision. `px` would pin the measure to a physical
        // width, so raising the font size would cut the characters per line. `rem`
        // tracks the root font-size — so it survives a size change — but it is blind
        // to the font *family*, which 5g is about to make user-settable. `ch` is the
        // width of a `0` in the font actually in use, so it is the only one of the
        // three that keeps the measure constant in characters under both settings.
        let (_, value) = settings
            .css_vars()
            .into_iter()
            .find(|(name, _)| *name == "--USER__maxLineLength")
            .expect("the measure is pushed");

        assert!(
            value.ends_with("ch"),
            "the measure is in {value}, not characters",
        );
    }

    #[test]
    fn the_font_family_reaches_the_layer_as_a_stack() {
        let settings = Settings {
            font_family: FontFamily::Sans,
            ..Settings::default()
        };

        let stack = FontFamily::Sans.stack();

        assert!(settings
            .css_vars()
            .contains(&("--USER__fontFamily", stack.to_string())));

        let layer = settings.user_layer();

        assert!(
            layer.contains(&format!("--USER__fontFamily: {stack};")),
            "the chosen face never reached the :root block",
        );
        assert!(
            layer.contains("font-family: var(--USER__fontFamily)"),
            "the layer declares a family it never applies — the picker would move \
             and the text would not",
        );
    }

    #[test]
    fn every_stack_ends_in_a_generic_family() {
        // The only link in the chain that cannot miss. Without it, a machine with none
        // of the named faces installed falls back to the UA default — which is the font
        // the book was already showing, so the setting silently does nothing there.
        for family in FontFamily::ALL
            .into_iter()
            .filter(|f| !f.stack().is_empty())
        {
            let stack = family.stack();
            let last = stack
                .rsplit(',')
                .next()
                .expect("a stack is non-empty")
                .trim();

            assert!(
                matches!(last, "serif" | "sans-serif" | "monospace"),
                "{family:?} ends in `{last}`, which is a face and might not exist",
            );
        }
    }

    #[test]
    fn no_stack_quotes_a_family_with_double_quotes() {
        // The stack does not only travel as CSS. `inline_styles()` puts it in a `style="…"`
        // attribute on the reader's own chrome, where a `"` closes the attribute early and
        // takes the rest of the declarations with it. Single quotes are legal CSS and have
        // no such second job.
        for family in FontFamily::ALL {
            assert!(
                !family.stack().contains('"'),
                "{family:?} quotes with `\"`, which cannot survive an HTML attribute",
            );
        }
    }

    #[test]
    fn the_monospace_elements_keep_their_own_font() {
        // `code`, `kbd`, `samp` and `pre` are monospace because the *content* is
        // column-aligned, not because the author had a taste in fonts. Overriding them
        // with a proportional face is how a code sample or an ASCII table stops being
        // readable — a change nobody asked for and nobody can undo.
        let layer = Settings::default().user_layer();
        let rule = layer
            .split('\n')
            .find(|rule| rule.contains("font-family:"))
            .expect("the layer applies the family");

        for tag in ["code", "kbd", "samp", "pre", "var"] {
            assert!(
                rule.contains(&format!(":not({tag})")),
                "the family lands on <{tag}>, whose font is structural",
            );
        }
    }

    #[test]
    fn the_publisher_is_the_default_font() {
        // The whole point of the gate. An override that is on by default is not a gate,
        // it is 5g — and 5g clobbers embedded fonts, which is the regression this closes.
        assert_eq!(Settings::default().font_family, FontFamily::Publisher);
    }

    #[test]
    fn the_publisher_pushes_an_empty_value_and_declares_nothing() {
        let settings = Settings {
            font_family: FontFamily::Publisher,
            ..Settings::default()
        };

        // Pushed, so a *live* switch back to the book's own face reaches the frame …
        assert!(settings
            .css_vars()
            .contains(&("--USER__fontFamily", String::new())));

        // … and not declared, because `--USER__fontFamily: ;` is not a declaration.
        assert!(
            !settings.vars().contains("--USER__fontFamily"),
            "the :root block declares a face under Publisher — the gate would open \
             the moment anything copied that block into an inline style",
        );
    }

    #[test]
    fn every_selector_on_the_font_rule_carries_the_gate() {
        // A selector *list* is the trap: prefixing the gate onto the first selector and
        // not the second leaves `body *` matching unconditionally, and the bug shows up
        // only on the descendants — i.e. on every paragraph, which is all the text there
        // is. Check each comma-separated selector, not the rule.
        let layer = Settings::default().user_layer();
        let rule = layer
            .split('\n')
            .find(|rule| rule.contains("font-family:"))
            .expect("the layer applies the family");
        let (selectors, _) = rule.split_once('{').expect("a rule has a block");

        for selector in selectors.split(',') {
            assert!(
                selector.contains("[style*='--USER__fontFamily']"),
                "`{}` overrides the font unconditionally",
                selector.trim(),
            );
        }
    }

    #[test]
    fn a_chosen_face_reaches_a_chapter_that_has_not_loaded_yet() {
        // The gate reads the inline style; serve-time injection writes a stylesheet rule.
        // Without this bootstrap the first paint of every new chapter ignores the setting.
        let settings = Settings {
            font_family: FontFamily::Sans,
            ..Settings::default()
        };
        let bootstrap = settings.bootstrap_js();

        assert!(bootstrap.contains("setProperty"));
        assert!(bootstrap.contains("--USER__fontFamily"));
        assert!(bootstrap.contains(FontFamily::Sans.stack()));

        // The stack is interpolated into a JavaScript string literal here, which is the
        // second job of `no_stack_quotes_a_family_with_double_quotes`: a `"` in a stack
        // would close the literal and turn the document into a syntax error.
        assert!(!FontFamily::Sans.stack().contains('"'));
    }

    #[test]
    fn the_publisher_bootstraps_nothing() {
        // Not "sets it to empty" — emits no script at all. There is nothing to undo on a
        // document that was born without the property.
        assert!(Settings::default().bootstrap_js().is_empty());
    }

    #[test]
    fn a_font_family_survives_a_slug_round_trip() {
        // Step 6 stores the choice as this slug. A variant that does not come back is a
        // setting that silently resets to the default on the next launch.
        for family in FontFamily::ALL {
            assert_eq!(FontFamily::from_slug(family.slug()), family);
        }

        assert_eq!(FontFamily::from_slug("comic-sans"), FontFamily::default());
    }

    #[test]
    fn a_theme_survives_a_slug_round_trip_and_the_slugs_are_distinct() {
        for theme in Theme::ALL {
            assert_eq!(Theme::from_slug(theme.slug()), theme);
        }

        assert_eq!(Theme::from_slug("solarized"), Theme::default());

        let slugs: std::collections::HashSet<&str> =
            Theme::ALL.iter().map(|theme| theme.slug()).collect();
        assert_eq!(
            slugs.len(),
            Theme::ALL.len(),
            "the picker marks the selected option by slug, so a shared slug would tick the wrong row",
        );
    }

    #[test]
    fn the_measure_steps_and_clamps() {
        let mut settings = Settings {
            max_line_length: 70,
            ..Settings::default()
        };

        settings.shorter();
        assert_eq!(settings.max_line_length, 70 - MAX_LINE_LENGTH_STEP);
        settings.longer();
        assert_eq!(settings.max_line_length, 70);

        for _ in 0..20 {
            settings.shorter();
        }
        assert_eq!(settings.max_line_length, MAX_LINE_LENGTH_MIN);

        for _ in 0..20 {
            settings.longer();
        }
        assert_eq!(settings.max_line_length, MAX_LINE_LENGTH_MAX);
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
