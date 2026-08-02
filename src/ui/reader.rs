use std::rc::Rc;

use dioxus::core::use_hook_with_cleanup; // not re-exported through the prelude
use dioxus::prelude::*;

use crate::{
    epub,
    nav::{self, ReaderDataStoreExt, ReaderState},
    ui::library::OpenBook,
};

const BRIDGE_JS: &str = include_str!("../web/assets/ook-events-listener.js");
const CHAPTER_LOADER_JS: &str = include_str!("../web/assets/chapter-loader.js");
const BLOB_CLEANUP_JS: &str = include_str!("../web/assets/blob-cleanup.js");

#[derive(Debug, PartialEq)]
pub(crate) enum BridgeMsg {
    Link(String),
    Scroll(usize),
    Pages(usize),
    Position(String),
}

impl BridgeMsg {
    fn parse(msg: &str) -> Option<BridgeMsg> {
        if let Some(href) = msg.strip_prefix("link:") {
            Some(BridgeMsg::Link(href.to_string()))
        } else if let Some(page) = msg.strip_prefix("scroll:") {
            page.parse().ok().map(BridgeMsg::Scroll)
        } else if let Some(page_count) = msg.strip_prefix("pages:") {
            page_count.parse().ok().map(BridgeMsg::Pages)
        } else if let Some(selector) = msg.strip_prefix("position:") {
            (!selector.is_empty()).then(|| BridgeMsg::Position(selector.to_string()))
        } else {
            None
        }
    }
}

#[component]
pub(crate) fn Reader(book: OpenBook) -> Element {
    epub::use_register_asset_handler(book.epub.clone());

    let mut open_book = use_context::<Signal<Option<OpenBook>>>();
    let docs = book.docs;
    let state = nav::use_reader_state(docs.len());
    let chapter = state.data.chapter();
    let pending_fragment = state.data.pending_fragment();
    let pending_last = state.data.pending_last();
    let (page, page_count) = (state.data.page(), state.data.page_count());
    let docs_for_iframe = docs.clone();

    let page_label = format!("Page {} of {}", page() + 1, page_count());
    let chapter_label = format!("Chapter {} of {}", chapter() + 1, state.chapter_count);

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
        let fragment = pending_fragment();
        let loader = document::eval(CHAPTER_LOADER_JS);
        _ = loader.send((url, fragment));
    });

    use_revoke_blob_on_unmount();
    use_bridge(state, docs);

    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100vh;",
            p {
                style: "text-align: center",
                "{book.title}"
            }

            iframe {
                id: "reader-frame",
                "sandbox": "allow-same-origin allow-scripts",
                style: "flex: 1; width: 100%; border: none;",
                class: if pending_last() || pending_fragment().is_some() { "invisible" },
            }

            div {
                style: "position: absolute; top: 8px; left: 8px;",
                button {
                    onclick: move |_| open_book.set(None),

                    "Close"
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

fn use_bridge(state: ReaderState, docs: Rc<Vec<String>>) {
    use_future(move || {
        let docs = docs.clone();
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
                    Some(BridgeMsg::Position(selector)) => state.on_position(selector),
                    None => {}
                }
            }
        }
    });
}

#[cfg(test)]
mod test {
    use super::*;

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
    fn the_loader_and_the_cleanup_agree_on_where_the_blob_url_lives() {
        // Two separate files sharing one global by name: rename it in the loader
        // and the cleanup silently revokes nothing, leaking a chapter per book.
        assert!(CHAPTER_LOADER_JS.contains("window.__ookBlobUrl"));
        assert!(BLOB_CLEANUP_JS.contains("window.__ookBlobUrl"));
        assert!(BLOB_CLEANUP_JS.contains("revokeObjectURL"));
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
}
