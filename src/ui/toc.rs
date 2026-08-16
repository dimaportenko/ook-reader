use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_primitives::ContentAlign;

use crate::{
    epub::LinkTarget,
    toc::{self, TocEntry},
    ui::components::popover::{PopoverContent, PopoverRoot, PopoverTrigger},
};

#[css_module("/src/ui/toc.css")]
struct Styles;

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
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    width: "24",
                    height: "24",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    class: "icon icon-tabler icons-tabler-outline icon-tabler-list",
                    path {
                        stroke: "none",
                        d: "M0 0h24v24H0z",
                        fill: "none",
                    }
                    path {
                        d: "M9 6l11 0",
                    }
                    path {
                        d: "M9 12l11 0",
                    }
                    path {
                        d: "M9 18l11 0",
                    }
                    path {
                        d: "M5 6l0 .01",
                    }
                    path {
                        d: "M5 12l0 .01",
                    }
                    path {
                        d: "M5 18l0 .01",
                    }
                }
            }
            PopoverContent {
                class: Styles::contents_popover__content.to_string(),
                align: ContentAlign::End,
                nav {
                    class: "{Styles::contents_popover__list}",
                    onkeydown: move |e| e.stop_propagation(),
                    for (index, entry) in entries.iter().enumerate() {
                        button {
                            class: "{Styles::contents_popover__entry}",
                            aria_current: if Some(index) == current { "page" },
                            style: "--toc-depth: {entry.depth};",
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
