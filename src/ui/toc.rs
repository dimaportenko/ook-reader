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
            open: open(),
            on_open_change: move |v| open.set(v),
            PopoverTrigger {
                Icon { icon: icon::LIST }
            }
            PopoverContent {
                class: Styles::contents_popover__content.to_string(),
                align: ContentAlign::End,
                nav {
                    class: "{Styles::contents_popover__list}",
                    for (index, entry) in entries.iter().enumerate() {
                        button {
                            class: "{Styles::contents_popover__entry}",
                            aria_current: if Some(index) == current { "page" },
                            style: "{DEPTH_VAR}: {entry.depth};",
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

#[cfg(test)]
mod test {
    use super::*;

    const TOC_CSS: &str = include_str!("toc.css");

    #[test]
    fn the_depth_variable_is_spelled_the_same_on_both_sides_of_the_css_gap() {
        assert!(TOC_CSS.contains(&format!("var({DEPTH_VAR}")));
    }
}
