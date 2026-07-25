const STYLE_OPEN: &str = "<style type=\"text/css\">\n/*<![CDATA[*/\n";
const STYLE_CLOSE: &str = "\n/*]]>*/\n</style>\n";

const SCRIPT_OPEN: &str = "<script type=\"text/javascript\">\n//<![CDATA[\n";
const SCRIPT_CLOSE: &str = "\n//]]>\n</script>\n";

pub(crate) fn wrap_css(css: &str) -> String {
    format!("{}{}{}", STYLE_OPEN, css, STYLE_CLOSE)
}

pub(crate) fn wrap_js(js: &str) -> String {
    format!("{}{}{}", SCRIPT_OPEN, js, SCRIPT_CLOSE)
}

pub(crate) const PAGINATION_CSS: &str = include_str!("./assets/pagination.css");
pub(crate) const PAGE_LISTENER_JS: &str = include_str!("./assets/page-listener.js");
pub(crate) const LINK_BRIDGE_JS: &str = include_str!("./assets/link-bridge.js");
pub(crate) const PAGE_COUNT_JS: &str = include_str!("./assets/page-count.js");
