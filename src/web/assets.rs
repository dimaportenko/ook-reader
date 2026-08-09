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

pub(crate) fn wrap_js_str(js: &str) -> String {
    format!("<script type=\"text/javascript\">\n//<![CDATA[\n{js}\n//]]>\n</script>\n")
}

#[cfg(test)]
mod test {
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
    fn the_reflow_handler_reuses_the_position_helpers() {
        // The anchor has to be the same notion of "where I am" that page-position.js
        // persists, which means the same code and not a second copy of it. A copy would
        // drift the first time one of the two is fixed.
        assert_eq!(INJECTED_ASSETS.matches("function selectorFor").count(), 1);
        assert_eq!(
            INJECTED_ASSETS
                .matches("function firstElementOnPage")
                .count(),
            1
        );
        assert_eq!(INJECTED_ASSETS.matches("const report =").count(), 1);
    }

    #[test]
    fn the_page_geometry_derives_from_one_column_width() {
        // padding, column-width and column-gap are one number wearing three hats: the
        // column plus one gap has to advance exactly 100vw, because that is the step
        // `translateX(calc(var(--ook-page) * -100vw))` moves by and the unit `pageOf`
        // divides by. Deriving all three from `--ook-column` is what keeps them in step
        // when the margin moves; three separate expressions would drift silently.
        assert!(
            INJECTED_ASSETS.contains("--ook-column:"),
            "no derived column width — the geometry is still three loose numbers",
        );
        assert_eq!(
            INJECTED_ASSETS.matches("var(--ook-column)").count(),
            3,
            "padding, column-width and column-gap each derive from the column, \
             or one of them is still hard-coded",
        );
    }

    #[test]
    fn the_measure_caps_the_column_alone() {
        let column = INJECTED_ASSETS
            .split_once("--ook-column:")
            .and_then(|(_, rest)| rest.split_once(';'))
            .map(|(value, _)| value)
            .expect("pagination.css declares the derived column");

        assert!(
            column.contains("min("),
            "the measure has to cap the column, and a cap is a min()",
        );
        assert!(
            column.contains("var(--USER__maxLineLength"),
            "the column ignores the measure entirely",
        );

        // One reference, in the column's own definition. Cap the padding or the gap
        // separately and they stop being "the leftover" — the advance stops being
        // 100vw and the transform drifts from `pageOf` again, which is the exact bug
        // 5e was built to make unreachable.
        assert_eq!(
            INJECTED_ASSETS
                .matches("var(--USER__maxLineLength")
                .count(),
            1,
            "the cap belongs in one place — every other number derives from it",
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
}

