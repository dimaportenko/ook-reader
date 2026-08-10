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
    wrap_js!("./assets/warn.js"),
    wrap_js!("./assets/page-geometry.js"),
    wrap_js!("./assets/page-listener.js"),
    wrap_js!("./assets/link-bridge.js"),
    wrap_js!("./assets/key-listener.js"),
    wrap_js!("./assets/page-count.js"),
    wrap_js!("./assets/fragment-scroll.js"),
    wrap_js!("./assets/page-position.js"),
    wrap_js!("./assets/reanchor.js"),
    wrap_js!("./assets/theme-listener.js"),
    wrap_js!("./assets/settle.js"),
    wrap_js!("./assets/boot.js"),
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

        // A theme push and a window resize end the same way: measure the anchor
        // against the new layout and tell Rust if the page moved. One
        // implementation, two callers — a second copy would fix one and not the
        // other, which is exactly how `resize` came to move the page silently.
        assert_eq!(INJECTED_ASSETS.matches("function reflowFrom").count(), 1);
        assert_eq!(INJECTED_ASSETS.matches("reflowFrom(").count(), 3);
        assert_eq!(INJECTED_ASSETS.matches("ook-reflow").count(), 1);
    }

    #[test]
    fn a_resize_measures_from_an_anchor_that_predates_the_reflow() {
        // Where the two paths differ. A theme push captures its anchor and *then*
        // mutates, so the capture sees the old layout. `resize` fires after the
        // layout has already changed, so capturing inside the handler asks "which
        // page is the element that is on page 4?" and always answers 4 — the
        // re-anchor becomes a tautology that can never report a move.
        //
        // The anchor therefore has to predate the reflow, and one already does:
        // the selector `reportPosition` computed on the last page change.
        assert_eq!(
            INJECTED_ASSETS.matches("function rememberAnchor").count(),
            1
        );
        assert_eq!(INJECTED_ASSETS.matches("rememberAnchor(").count(), 2);

        // Only the capture-then-mutate path may look the anchor up live.
        assert_eq!(
            INJECTED_ASSETS.matches("firstElementOnPage(").count(),
            3,
            "one definition, `reportPosition`, and `reanchor` — a fourth caller is \
             probably a resize handler capturing an anchor it cannot trust",
        );

        // A drag fires dozens of events. Undebounced, each one walks the whole
        // document, and each intermediate reflow round-trips through Rust as a
        // `set-page` whose `reportPosition` overwrites the anchor with the top of
        // the new page — so the position slides backwards across the gesture.
        assert!(INJECTED_ASSETS.contains("RESIZE_SETTLE_MS"));
        assert!(INJECTED_ASSETS.contains("clearTimeout"));
    }

    #[test]
    fn a_reflow_echo_does_not_re_derive_the_anchor() {
        // `reportPosition` computes "first element on page N", for persistence.
        // `lastAnchor` is "the element the reader is on". They agree at a page turn
        // and diverge after a reflow, because the top of the page the reflow moved
        // you to is *earlier* content than the element you were anchored to.
        //
        // Letting the echoed `set-page` re-derive the anchor ratchets the reader
        // backwards once per reflow — measured 57 → 53 → 46 → 42 → 31 → 21 → 15
        // over one round trip out to a narrow window and back to the same size.
        assert_eq!(INJECTED_ASSETS.matches("function isReflowEcho").count(), 1);
        assert_eq!(INJECTED_ASSETS.matches("isReflowEcho(").count(), 2);

        // Keyed on the page number, not a bare boolean: a `set-page` for some other
        // page is a real navigation and still has to re-anchor and save.
        assert!(INJECTED_ASSETS.contains("pendingReflowPage === page"));
    }

    #[test]
    fn the_first_measurement_waits_for_the_document_to_settle() {
        // The bug this exists to prevent: `load` fires before a book's embedded
        // @font-face files land, so the chapter is measured in a fallback font,
        // the restored page is committed, and the font swap then reflows the text
        // out from under it — one page behind, every time.
        assert!(INJECTED_ASSETS.contains("document.fonts"));
        assert!(
            INJECTED_ASSETS.contains("SETTLE_TIMEOUT_MS"),
            "a font that never loads must not wedge the reader shut",
        );

        // One `load` listener owns the whole opening sequence. Three files each
        // registering their own put the order at the mercy of the concat below,
        // and the count has to be reported before the fragment resolves against it.
        assert_eq!(
            INJECTED_ASSETS
                .matches(r#"addEventListener("load""#)
                .count(),
            1,
        );

        let boot = INJECTED_ASSETS
            .rsplit_once("whenSettled")
            .map(|(_, rest)| rest)
            .expect("boot.js runs the opening sequence behind the settle gate");
        let step = |name: &str| boot.find(name).expect("boot runs {name}");
        assert!(step("report()") < step("reportFragmentPage()"));
        assert!(step("reportFragmentPage()") < step("reportPosition("));
        assert!(step("reportPosition(") < step("ook-ready"));
    }

    #[test]
    fn the_settle_gate_is_defined_before_the_boot_sequence_uses_it() {
        // Both are top-level scripts in one document, so this is plain source
        // order — and `const SETTLE_TIMEOUT_MS` is in the temporal dead zone until
        // its own script has run.
        let define = INJECTED_ASSETS
            .find("function whenSettled")
            .expect("settle.js is injected");
        let call = INJECTED_ASSETS
            .rfind("whenSettled(")
            .expect("boot.js is injected");
        assert!(define < call);
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

