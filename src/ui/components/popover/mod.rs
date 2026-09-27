mod component;
pub use component::*;

#[cfg(test)]
mod test {
    const NARROW_MAX: &str = "40rem";
    const POPOVER_CSS: &str = include_str!("style.css");

    fn rule_body<'a>(css: &'a str, opener: &str) -> &'a str {
        css.split_once(opener)
            .unwrap_or_else(|| panic!("no rule opens with `{opener}`"))
            .1
            .split_once("\n}")
            .expect("an unclosed rule")
            .0
    }

    #[test]
    fn the_popover_is_bounded_by_the_viewport_and_not_by_its_trigger() {
        let base = rule_body(POPOVER_CSS, ".dx-popover-content {");

        assert!(
            !base.contains("max-width: calc(100%"),
            "every [data-side] rule re-positions the panel to absolute, where a \
             percentage max-width resolves against the 40px trigger",
        );
        assert!(
            base.contains("dvw"),
            "only a viewport unit means the same thing under both position \
             schemes the rules disagree about",
        );
    }

    #[test]
    fn the_panel_becomes_a_sheet_below_the_width_the_popover_widens_at() {
        assert!(POPOVER_CSS.contains(&format!("@media (width >= {NARROW_MAX})")));

        let sheet = rule_body(POPOVER_CSS, &format!("@media (width < {NARROW_MAX}) {{"));

        assert!(
            sheet.contains("position: fixed"),
            "a narrow viewport pins the panel to the screen, not to its trigger",
        );
        assert!(
            sheet.contains("min-width: 0"),
            "a popover's own min-width floor would outgrow the viewport the sheet is pinned to",
        );
    }
}
