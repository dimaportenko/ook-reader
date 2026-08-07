macro_rules! wrap_css {
    ($path:literal) => {
        concat!(
            "<style type=\"text/css\">\n/*<![CDATA[*/\n",
            include_str!($path),
            "\n/*]]>*/\n</style>\n"
        )
    };
}

macro_rules! wrap_js {
    ($path:literal) => {
        concat!(
            "<script type=\"text/javascript\">\n//<![CDATA[\n",
            include_str!($path),
            "\n//]]>\n</script>\n"
        )
    };
}

pub(crate) const INJECTED_ASSETS: &str = concat!(
    wrap_css!("./assets/pagination.css"),
    wrap_js!("./assets/page-geometry.js"),
    wrap_js!("./assets/page-listener.js"),
    wrap_js!("./assets/link-bridge.js"),
    wrap_js!("./assets/page-count.js"),
    wrap_js!("./assets/fragment-scroll.js"),
    wrap_js!("./assets/page-position.js"),
    wrap_js!("./assets/theme-listener.js"),
);

pub(crate) const READING_SYSTEM_DEFAULTS: &str = wrap_css!("./assets/reading-system.css");

pub(crate) fn wrap_css_str(css: &str) -> String {
    format!("<style type=\"text/css\">\n/*<![CDATA[*/\n{css}\n/*]]>*/\n</style>\n")
}

#[cfg(test)]
mod test {
    use crate::web::theme::Theme;

    use super::*;

    #[test]
    fn the_page_formula_is_defined_once_across_the_injected_assets() {
        assert_eq!(INJECTED_ASSETS.matches("function currentPage").count(), 1);
        assert_eq!(
            INJECTED_ASSETS
                .matches("el.offsetLeft / window.innerWidth")
                .count(),
            1
        );
    }

    #[test]
    fn wrapped_css_is_a_cdata_style_element() {
        let out = wrap_css_str("body > p { color: red }");

        assert!(out.starts_with("<style type=\"text/css\">"));
        assert!(out.trim_end().ends_with("</style>"));
        assert!(
            out.contains("/*<![CDATA[*/") && out.contains("/*]]>*/"),
            "an unescaped > in a selector aborts the whole XHTML document without CDATA",
        );
        assert!(out.contains("body > p { color: red }"));
    }

    #[test]
    fn every_theme_sets_both_user_colour_variables() {
        for theme in [Theme::Day, Theme::Sepia, Theme::Night] {
            let css = theme.vars();

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
            let css = theme.user_layer();
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
            let layer = theme.user_layer();

            // Nothing pushed that the served layer never declares or never applies …
            for (name, value) in theme.css_vars() {
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
                    theme.css_vars().iter().any(|(pushed, _)| *pushed == name),
                    "the layer reads {name}, which the message never sets — \
                     that variable would only ever update on a chapter turn",
                );
            }
        }
    }

    #[test]
    fn the_three_themes_are_actually_different() {
        assert_ne!(Theme::Day.vars(), Theme::Night.vars());
        assert_ne!(Theme::Day.vars(), Theme::Sepia.vars());
        assert_ne!(Theme::Sepia.vars(), Theme::Night.vars());
    }
}
