use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_primitives::ContentAlign;

use crate::{
    epub::LinkTarget,
    toc::{self, TocEntry},
    ui::components::{
        icon::{self, Icon},
        popover::{PopoverContent, PopoverRoot, PopoverTrigger},
    },
};

#[css_module("/src/ui/toc.css")]
struct Styles;

const DEPTH_VAR: &str = "--toc-depth";

#[component]
pub(crate) fn ContentsPopover(
    entries: Rc<Vec<TocEntry>>,
    chapter: usize,
    on_pick: EventHandler<LinkTarget>,
) -> Element {
    let mut open = use_signal(|| false);

    if entries.is_empty() {
        return rsx! {};
    }

    let current = toc::entry_index_for_spine(&entries, chapter);

    rsx! {
        PopoverRoot {
            is_modal: false,
            open: open(),
            on_open_change: move |v| open.set(v),
            PopoverTrigger {
                aria_label: "Table of contents",
                Icon { icon: icon::LIST }
            }
            PopoverContent {
                class: Styles::contents_popover__content.to_string(),
                align: ContentAlign::End,
                nav {
                    aria_label: "Table of contents",
                    ul {
                        class: "{Styles::contents_popover__list}",
                        for (index, entry) in entries.iter().enumerate() {
                            li {
                                button {
                                    class: "{Styles::contents_popover__entry}",
                                    aria_current: if Some(index) == current { "page" },
                                    style: "{DEPTH_VAR}: {entry.depth};",
                                    onmounted: move |e| async move {
                                        if Some(index) == current {
                                            if let Err(err) = e.scroll_to(ScrollBehavior::Instant).await {
                                                eprintln!("ook: the contents panel did not scroll: {err}");
                                            }
                                        }
                                    },
                                    onclick: {
                                        let target = entry.target.clone();

                                        move |e| {
                                            e.stop_propagation();
                                            open.set(false);
                                            on_pick.call(target.clone());
                                        }
                                    },
                                    "{entry.label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const NARROW_MAX: &str = "40rem";
    const TOC_CSS: &str = include_str!("toc.css");
    const POPOVER_CSS: &str = include_str!("components/popover/style.css");

    #[test]
    fn the_depth_variable_is_spelled_the_same_on_both_sides_of_the_css_gap() {
        assert!(TOC_CSS.contains(&format!("var({DEPTH_VAR}")));
    }

    #[test]
    fn the_popover_is_bounded_by_the_viewport_and_not_by_its_trigger() {
        let base = POPOVER_CSS
            .split_once(".dx-popover-content {")
            .expect("the rule every popover starts from")
            .1
            .split_once('}')
            .expect("an unclosed rule")
            .0;

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

        let sheet = TOC_CSS
            .split_once(&format!("@media (width < {NARROW_MAX})"))
            .expect("the contents panel has a narrow-viewport rule")
            .1;

        assert!(
            sheet.contains("min-width: 0"),
            "the 24rem floor outgrows the viewport the sheet is pinned to",
        );
    }
}
