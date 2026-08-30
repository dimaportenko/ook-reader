use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct TablerIcon {
    name: &'static str,
    paths: &'static [&'static str],
}

pub(crate) const CLOSE: TablerIcon = TablerIcon {
    name: "x",
    paths: &["M18 6l-12 12", "M6 6l12 12"],
};

pub(crate) const CHEVRON_LEFT: TablerIcon = TablerIcon {
    name: "chevron-left",
    paths: &["M15 6l-6 6l6 6"],
};

pub(crate) const CHEVRON_RIGHT: TablerIcon = TablerIcon {
    name: "chevron-right",
    paths: &["M9 6l6 6l-6 6"],
};

pub(crate) const SETTINGS: TablerIcon = TablerIcon {
    name: "settings",
    paths: &[
        "M10.325 4.317c.426 -1.756 2.924 -1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543 -.94 3.31 .826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.756 .426 1.756 2.924 0 3.35a1.724 1.724 0 0 0 -1.066 2.573c.94 1.543 -.826 3.31 -2.37 2.37a1.724 1.724 0 0 0 -2.572 1.065c-.426 1.756 -2.924 1.756 -3.35 0a1.724 1.724 0 0 0 -2.573 -1.066c-1.543 .94 -3.31 -.826 -2.37 -2.37a1.724 1.724 0 0 0 -1.065 -2.572c-1.756 -.426 -1.756 -2.924 0 -3.35a1.724 1.724 0 0 0 1.066 -2.573c-.94 -1.543 .826 -3.31 2.37 -2.37c1 .608 2.296 .07 2.572 -1.065",
        "M9 12a3 3 0 1 0 6 0a3 3 0 0 0 -6 0",
    ],
};

pub(crate) const LIST: TablerIcon = TablerIcon {
    name: "list",
    paths: &[
        "M9 6l11 0",
        "M9 12l11 0",
        "M9 18l11 0",
        "M5 6l0 .01",
        "M5 12l0 .01",
        "M5 18l0 .01",
    ],
};

pub(crate) const ADD: TablerIcon = TablerIcon {
    name: "plus",
    paths: &["M12 5l0 14", "M5 12l14 0"],
};

pub(crate) const EDIT: TablerIcon = TablerIcon {
    name: "edit",
    paths: &[
        "M7 7h-1a2 2 0 0 0 -2 2v9a2 2 0 0 0 2 2h9a2 2 0 0 0 2 -2v-1",
        "M20.385 6.585a2.1 2.1 0 0 0 -3 -3l-9.385 9.415v3h3l9.385 -9.415z",
        "M16 5l3 3",
    ],
};

pub(crate) const TRASH: TablerIcon = TablerIcon {
    name: "trash",
    paths: &[
        "M4 7l16 0",
        "M10 11l0 6",
        "M14 11l0 6",
        "M5 7l1 12a2 2 0 0 0 2 2h8a2 2 0 0 0 2 -2l1 -12",
        "M9 7v-3a1 1 0 0 1 1 -1h4a1 1 0 0 1 1 1v3",
    ],
};

#[component]
pub(crate) fn Icon(icon: TablerIcon) -> Element {
    rsx! {
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
            class: "icon icon-tabler icons-tabler-outline icon-tabler-{icon.name}",
            path {
                stroke: "none",
                d: "M0 0h24v24H0z",
                fill: "none",
            }
            for d in icon.paths.iter().copied() {
                path { d }
            }
        }
    }
}
