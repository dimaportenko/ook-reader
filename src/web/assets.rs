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
}

