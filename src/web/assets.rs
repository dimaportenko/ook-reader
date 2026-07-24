const STYLE_OPEN: &str = "<style type=\"text/css\">\n/*<![CDATA[*/\n";
const STYLE_CLOSE: &str = "\n/*]]>*/\n</style>\n";

const SCRIPT_OPEN: &str = "<script type=\"text/javascript\">\n//<![CDATA[\n";
const SCRIPT_CLOSE: &str = "\n//]]>\n</script>\n";

pub(crate) fn get_wrapped_css(css: &str) -> String {
    format!("{} {} {}", STYLE_OPEN, css, STYLE_CLOSE)
}

pub(crate) fn get_wrapped_js(js: &str) -> String {
    format!("{} {} {}", SCRIPT_OPEN, js, SCRIPT_CLOSE)
}
