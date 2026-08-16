use std::rc::Rc;

use dioxus::core::use_hook_with_cleanup; // not re-exported through the prelude
use dioxus::prelude::*;

use crate::{
    clock::now_secs,
    epub::{self, Locator},
    library::Library,
    nav::{self, ReaderDataStoreExt, ReaderState},
    settings::Settings,
    toc::{self, TocEntry},
    ui::{library::OpenBook, settings::SettingsPopover, OrLog},
};

const BRIDGE_JS: &str = include_str!("../web/assets/ook-events-listener.js");
const CHAPTER_LOADER_JS: &str = include_str!("../web/assets/chapter-loader.js");
const BLOB_CLEANUP_JS: &str = include_str!("../web/assets/blob-cleanup.js");
const THEME_PUSH_JS: &str = include_str!("../web/assets/theme-push.js");

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Turn {
    Prev,
    Next,
}

impl Turn {
    fn of(key: &Key) -> Option<Turn> {
        match key {
            Key::ArrowLeft => Some(Turn::Prev),
            Key::ArrowRight => Some(Turn::Next),
            _ => None,
        }
    }

    fn apply(self, state: ReaderState) {
        match self {
            Turn::Prev => state.page_prev(),
            Turn::Next => state.page_next(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum BridgeMsg {
    Link(String),
    Scroll(usize),
    Pages(usize),
    Position(String),
    Reflow(usize),
    Turn(Turn),
    Ready,
    Warn(String),
}

impl BridgeMsg {
    fn parse(msg: &str) -> Option<BridgeMsg> {
        if let Some(href) = msg.strip_prefix("link:") {
            Some(BridgeMsg::Link(href.to_string()))
        } else if let Some(key) = msg.strip_prefix("key:") {
            key.parse::<Key>()
                .ok()
                .as_ref()
                .and_then(Turn::of)
                .map(BridgeMsg::Turn)
        } else if let Some(page) = msg.strip_prefix("scroll:") {
            page.parse().ok().map(BridgeMsg::Scroll)
        } else if let Some(page_count) = msg.strip_prefix("pages:") {
            page_count.parse().ok().map(BridgeMsg::Pages)
        } else if let Some(selector) = msg.strip_prefix("position:") {
            (!selector.is_empty()).then(|| BridgeMsg::Position(selector.to_string()))
        } else if let Some(page) = msg.strip_prefix("reflow:") {
            page.parse().ok().map(BridgeMsg::Reflow)
        } else if msg == "ready:" {
            Some(BridgeMsg::Ready)
        } else {
            msg.strip_prefix("warn:")
                .map(|message| BridgeMsg::Warn(message.to_string()))
        }
    }
}

fn chapter_label(entries: &[TocEntry], chapter: usize, chapter_count: usize) -> String {
    match toc::entry_index_for_spine(entries, chapter) {
        Some(index) => entries[index].label.clone(),
        None => format!("Chapter {} of {}", chapter + 1, chapter_count),
    }
}

fn page_label(page: usize, count: usize) -> String {
    match count {
        0 => "Page …".to_string(),
        count => format!("Page {} of {}", page + 1, count),
    }
}

#[component]
pub(crate) fn Reader(book: OpenBook) -> Element {
    let settings = use_context::<Signal<Settings>>();
    epub::use_register_asset_handler(book.epub.clone(), settings());

    let library = use_context::<Rc<Library>>();
    let mut open_book = use_context::<Signal<Option<OpenBook>>>();
    let docs = book.docs;
    let entries = use_hook(|| Rc::new(toc::toc_entries(&book.epub, &docs)));
    let start = use_hook(|| {
        library
            .position(book.id)
            .or_log("read the reading position")
            .flatten()
    });
    let state = nav::use_reader_state(docs.len(), start);
    let chapter = state.data.chapter();
    let pending = state.data.pending();
    let (page, page_count) = (state.data.page(), state.data.page_count());
    let hidden = nav::chapter_is_hidden(state.data.phase()(), &pending());
    let docs_for_iframe = docs.clone();

    let page_label = page_label(page(), page_count());
    let chapter_label = chapter_label(&entries, chapter(), state.chapter_count);

    use_effect(move || {
        let push = document::eval(THEME_PUSH_JS);
        _ = push.send(settings().css_vars());
    });

    use_effect(move || {
        let page_number = page();
        let script = format!(
            r#"
            const iframe = document.getElementById("reader-frame");
            if (iframe && iframe.contentWindow) {{
                iframe.contentWindow.postMessage(
                    {{ kind: "ook-set-page", page: {} }},
                    "*"
                );
            }}
        "#,
            page_number
        );
        document::eval(&script);
    });

    use_effect(move || {
        let url = epub::chapter_url(&docs_for_iframe[chapter()]);
        let loader = document::eval(CHAPTER_LOADER_JS);
        _ = loader.send((url, pending().fragment()));
    });

    use_revoke_blob_on_unmount();
    use_bridge(state, docs, library, book.id);

    rsx! {
        div {
            class: "reader-root",
            style: "display: flex; flex-direction: column; height: 100vh; {settings().inline_styles()}",
            tabindex: "0",
            onmounted: move |e| async move {
                _ = e.set_focus(true).await;
            },
            onkeydown: move |e| {
                if let Some(turn) = Turn::of(&e.key()) {
                    e.prevent_default();
                    turn.apply(state);
                }
            },

            div {
                style: "display: flex; justify-content: space-between;",
                div {
                    style: "padding: 0.75rem 1rem; z-index: 1;",

                    button {
                        class: "icon-button",
                        onclick: move |_| open_book.set(None),
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
                            class: "icon icon-tabler icons-tabler-outline icon-tabler-x",
                            path {
                                stroke: "none",
                                d: "M0 0h24v24H0z",
                                fill: "none",
                            }
                            path {
                                d: "M18 6l-12 12",
                            }
                            path {
                                d: "M6 6l12 12",
                            }
                        }
                    }
                }
                div {
                    style: "position: absolute; top: 0; left: 0; right: 0; padding: 0.5rem 1rem",
                    p {
                        style: "text-align: center; margin: 0.5rem 0 0;",
                        "{book.title}"
                    }

                    p {
                        style: "text-align: center; margin: 0.5rem 0 0;",
                        "{chapter_label}"
                    }
                }
                div {
                    style: "padding: 0.5rem 1rem; z-index: 1; display: flex; gap: 0.5rem;",
                    SettingsPopover {}
                }

            }

            div {
                style: "flex: 1; position: relative; display: flex;",

                iframe {
                    id: "reader-frame",
                    "sandbox": "allow-same-origin allow-scripts",
                    style: "flex: 1; width: 100%; border: none;",
                    class: if hidden { "invisible" },
                }

                if hidden {
                    div {
                        class: "reader-loading",
                        div {
                            class: "reader-loading__spinner",
                        }
                    }
                }
            }

            NavRow {
                on_prev: move |_| state.chapter_prev(),
                on_next: move |_| state.chapter_next(),
                label: chapter_label,
            }

            NavRow {
                on_prev: move |_| state.page_prev(),
                on_next: move |_| state.page_next(),
                label: page_label,
            }
        }
    }
}

#[component]
fn NavRow(
    label: String,
    on_next: EventHandler<MouseEvent>,
    on_prev: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div {
            style: "display: flex; gap: 8px; padding: 8px; justify-content: center;",
            button {
                onclick: move |e| on_prev.call(e),
                "Prev"
            }

            span {
                "{label}"
            }

            button {
                onclick: move |e| on_next.call(e),
                "Next"
            }
        }
    }
}

/// Revokes the last chapter's blob when the reader leaves the screen.
///
/// The window handle is captured at mount and carried into the cleanup rather
/// than looked up inside it. `document::eval` finds its provider through the
/// *current scope's* context, and a drop runs with no scope current — it would
/// quietly fall back to a no-op document and the script would never run. Going
/// through the webview directly also means holding a strong handle, so there is
/// no weak upgrade left to fail if the window is torn down first.
fn use_revoke_blob_on_unmount() {
    let window = dioxus::desktop::use_window();

    use_hook_with_cleanup(
        move || window,
        |window| {
            _ = window.webview.evaluate_script(BLOB_CLEANUP_JS);
        },
    );
}

fn use_bridge(state: ReaderState, docs: Rc<Vec<String>>, library: Rc<Library>, book_id: i64) {
    use_future(move || {
        let docs = docs.clone();
        let library = Rc::clone(&library);
        async move {
            let mut bridge = document::eval(BRIDGE_JS);

            while let Ok(msg) = bridge.recv::<String>().await {
                match BridgeMsg::parse(&msg) {
                    Some(BridgeMsg::Link(href)) => {
                        let idx = *state.data.chapter().peek();
                        if let Some(target) = epub::resolve_internal_link(&docs, idx, &href) {
                            state.follow_link(target);
                        }
                    }
                    Some(BridgeMsg::Scroll(page)) => state.on_scroll(page),
                    Some(BridgeMsg::Pages(p_count)) => state.on_pages(p_count),
                    Some(BridgeMsg::Position(selector)) => {
                        if state.data.pending().peek().is_settling() {
                            continue;
                        }
                        let locator = Locator {
                            spine_index: *state.data.chapter().peek(),
                            selector,
                        };
                        library
                            .save_position(book_id, &locator, now_secs())
                            .or_log("save the reading position");
                    }
                    Some(BridgeMsg::Reflow(page)) => state.on_reflow(page),
                    Some(BridgeMsg::Turn(turn)) => turn.apply(state),
                    Some(BridgeMsg::Ready) => state.on_ready(),
                    Some(BridgeMsg::Warn(message)) => eprintln!("ook: {message}"),
                    None => {}
                }
            }
        }
    });
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use super::*;

    #[test]
    fn the_chapter_label_prefers_the_toc_entry_over_the_ordinal() {
        let (epub, docs) =
            epub::open_with_spine(Path::new(crate::TEST_BOOK)).expect("open fixture book");
        let entries = toc::toc_entries(&epub, &docs);

        assert_eq!(
            chapter_label(&entries, 2, docs.len()),
            "I. A SCANDAL IN BOHEMIA"
        );

        assert_eq!(chapter_label(&entries, 0, docs.len()), "Chapter 1 of 15");

        assert_eq!(chapter_label(&[], 2, 15), "Chapter 3 of 15");
    }

    #[test]
    fn bridge_parses_each_message_kind() {
        assert_eq!(BridgeMsg::parse("scroll:3"), Some(BridgeMsg::Scroll(3)));
        assert_eq!(BridgeMsg::parse("pages:12"), Some(BridgeMsg::Pages(12)));
        assert_eq!(
            BridgeMsg::parse("link:chapter2.xhtml#s3"),
            Some(BridgeMsg::Link("chapter2.xhtml#s3".to_string()))
        );
        // unknown prefixes and malformed numbers decode to None, never panic
        assert_eq!(BridgeMsg::parse("scroll:notanumber"), None);
        assert_eq!(BridgeMsg::parse("bogus:1"), None);
    }

    #[test]
    fn the_page_label_waits_for_a_real_count() {
        assert_eq!(page_label(0, 0), "Page …");
        assert_eq!(page_label(3, 0), "Page …");

        assert_eq!(page_label(0, 12), "Page 1 of 12");
        assert_eq!(page_label(11, 12), "Page 12 of 12");
    }

    #[test]
    fn the_loader_and_the_cleanup_agree_on_where_the_blob_url_lives() {
        // Two separate files sharing one global by name: rename it in the loader
        // and the cleanup silently revokes nothing, leaking a chapter per book.
        assert!(CHAPTER_LOADER_JS.contains("window.__ookBlobUrl"));
        assert!(BLOB_CLEANUP_JS.contains("window.__ookBlobUrl"));
        assert!(BLOB_CLEANUP_JS.contains("revokeObjectURL"));
    }

    #[test]
    fn the_theme_push_and_the_chapter_listener_agree_on_the_message_kind() {
        // Two files, one message name, no compiler between them. Rename it on one side
        // and the theme silently stops arriving — nothing errors, the colours just stop.
        assert!(THEME_PUSH_JS.contains("ook-set-theme"));
        assert!(crate::web::assets::INJECTED_ASSETS.contains("ook-set-theme"));
    }

    #[test]
    fn an_empty_pushed_value_removes_the_property() {
        // `setProperty(name, "")` is a no-op, not a removal — the gate would stay open
        // and Publisher would silently do nothing. Rust decides empty means remove; only
        // this string in a JS file honours it, and no compiler sees both.
        assert!(crate::web::assets::INJECTED_ASSETS.contains("removeProperty"));
    }

    #[test]
    fn bridge_parses_a_position_selector_whole() {
        assert!(BRIDGE_JS.contains("ook-position"));

        // A selector is colon- and space-rich. `strip_prefix` hands back the entire
        // remainder of the message, so nothing here gets split in half.
        assert_eq!(
            BridgeMsg::parse("position:body > div:nth-child(2) > p:nth-child(7)"),
            Some(BridgeMsg::Position(
                "body > div:nth-child(2) > p:nth-child(7)".to_string()
            )),
        );

        // An empty payload is not a position. Reject it here rather than storing a
        // selector that can never resolve — Step 6 writes this straight to SQLite.
        assert_eq!(BridgeMsg::parse("position:"), None);
    }

    #[test]
    fn a_warning_from_the_frame_survives_all_three_hops() {
        // The frame is sandboxed, so its `console` goes to the webview's console and
        // not the terminal. This hop is the only way anything inside it can speak.
        // It carries failures only — a fragment that will not resolve, a position
        // that could not be saved, fonts that never finished — because a trace on
        // every page turn is how a real warning goes unread.
        assert!(crate::web::assets::INJECTED_ASSETS.contains("ook-warn"));
        assert!(BRIDGE_JS.contains("ook-warn"));

        // Same whole-remainder parse as `position:` — a warning is prose and will
        // contain colons.
        assert_eq!(
            BridgeMsg::parse("warn:no element on page 3, position not saved"),
            Some(BridgeMsg::Warn(
                "no element on page 3, position not saved".to_string()
            )),
        );
    }

    #[test]
    fn an_arrow_key_inside_the_frame_survives_all_three_hops() {
        // The frame owns the keyboard the moment it is clicked, so the host's own
        // `onkeydown` never sees an arrow pressed while reading. key-listener.js posts
        // it, ook-events-listener.js forwards it, `parse` reads it back — same three
        // files, same no-compiler-between-them hazard as `ook-reflow`.
        assert!(crate::web::assets::INJECTED_ASSETS.contains("ook-key"));
        assert!(BRIDGE_JS.contains("ook-key"));

        // The wire carries the DOM key name so both entry points end at one mapping:
        // `Turn::of` decides what an arrow means, whether the press arrived through
        // the bridge or through the host's `onkeydown`.
        assert_eq!(
            BridgeMsg::parse("key:ArrowLeft"),
            Some(BridgeMsg::Turn(Turn::Prev))
        );
        assert_eq!(
            BridgeMsg::parse("key:ArrowRight"),
            Some(BridgeMsg::Turn(Turn::Next))
        );

        // A key with no meaning here is dropped rather than parsed into a turn, and
        // an unrecognised name is not a panic.
        assert_eq!(BridgeMsg::parse("key:ArrowUp"), None);
        assert_eq!(BridgeMsg::parse("key:notakey"), None);
    }

    #[test]
    fn the_reflow_message_survives_all_three_hops() {
        // theme-listener.js posts it, ook-events-listener.js forwards it, `parse` reads
        // it back. Three files, one name, and no compiler between any two of them —
        // rename it in one and the count goes stale again, silently.
        assert!(crate::web::assets::INJECTED_ASSETS.contains("ook-reflow"));
        assert!(BRIDGE_JS.contains("ook-reflow"));

        assert_eq!(BridgeMsg::parse("reflow:7"), Some(BridgeMsg::Reflow(7)));
        assert_eq!(BridgeMsg::parse("reflow:notanumber"), None);
    }
}
