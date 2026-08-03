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
);

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
}
