#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::desktop::{use_asset_handler, wry::http::Response};
use dioxus::prelude::*;
use rbook::Epub;

mod epub;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
pub(crate) const BOOK: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/book/pg1661-adventures-of-sherlock-holmes.epub");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let epub = use_hook(|| Rc::new(Epub::open(BOOK).expect("should open the bundled epub")));

    use_asset_handler(epub::EPUB_ROUTE, move |request, responder| {
        let path = request
            .uri()
            .path()
            .strip_prefix(&format!("/{}", epub::EPUB_ROUTE))
            .unwrap_or_default();

        match epub.read_resource_bytes(path) {
            Ok(bytes) => {
                let body = Response::builder()
                    .header("Content-Type", epub::content_type_for(path))
                    .body(bytes)
                    .unwrap();
                responder.respond(body);
            }
            Err(_) => {
                let not_found = Response::builder().status(404).body(Vec::new()).unwrap();
                responder.respond(not_found);
            }
        }
    });

    rsx! {
        document::Link {
            rel: "icon",
            href: FAVICON,
        }
        document::Link {
            rel: "stylesheet",
            href: MAIN_CSS,
        }
        Reader {}
    }
}

#[component]
fn Reader() -> Element {
    let docs = use_hook(|| epub::load_spine(BOOK).expect("bundled epub should load"));
    let mut current = use_signal(|| 0usize);
    let mut page = use_signal(|| 0usize);
    let mut pending_fragment = use_signal(|| None::<String>);
    let len = docs.len();
    let current_doc = &docs[current()];
    let paged_doc = epub::inject_pagination_css(&current_doc.xhtml, page());
    let bridged = epub::inject_link_bridge(&paged_doc);
    let prepared = match pending_fragment() {
        Some(frag) => epub::inject_fragment_scroll(&bridged, &frag),
        None => bridged,
    };
    let iframe_src = epub::to_xhtml_data_url(&prepared);

    use_future(move || {
        let docs = docs.clone();
        async move {
            let mut bridge = document::eval(
                r#"
            window.addEventListener('message', (e) => {
                if (!e.data) return;
                if (e.data.kind === 'ook-link') {
                    dioxus.send("link:" + e.data.raw);
                }
                if (e.data.kind === 'ook-scroll') {
                    dioxus.send("scroll:" + e.data.page);
                }
            });
            "#,
            );

            while let Ok(msg) = bridge.recv::<String>().await {
                if let Some(href) = msg.strip_prefix("link:") {
                    let idx = *current.peek();
                    if let Some(target) = epub::resolve_internal_link(&docs, idx, href) {
                        current.set(target.spine_index);
                        page.set(0);
                        pending_fragment.set(target.fragment);
                    }
                } else if let Some(p) = msg.strip_prefix("scroll:") {
                    if let Ok(p) = p.parse::<usize>() {
                        page.set(p);
                        pending_fragment.set(None);
                    }
                }
            }
        }
    });

    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100vh;",

            iframe {
                "sandbox": "allow-same-origin allow-scripts",
                style: "flex: 1; width: 100%; border: none;",
                src: "{iframe_src}",
            }

            div {
                style: "display: flex; gap: 8px; padding: 8px; justify-content: center;",
                button {
                    onclick: move |_| {
                        page.set(0);
                        current.set(prev_index(current()));
                    },
                    "Prev"
                }

                span {
                    "Chapter {current()}"
                }

                button {
                    onclick: move |_| {
                        page.set(0);
                        current.set(next_index(current(), len));
                    },
                    "Next"
                }
            }

            div {
                style: "display: flex; gap: 8px; padding: 8px; justify-content: center;",
                button {
                    onclick: move |_| page.set(page().saturating_sub(1)),
                    "Prev"
                }

                span {
                    "Page {page() + 1}"
                }

                button {
                    onclick: move |_| page.set(page() + 1),
                    "Next"
                }
            }
        }
    }
}

fn prev_index(current: usize) -> usize {
    current.saturating_sub(1)
}

fn next_index(current: usize, len: usize) -> usize {
    (current + 1).min(len.saturating_sub(1))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn paging_clamps_at_both_ends() {
        let len = 15;

        assert_eq!(next_index(0, len), 1);

        assert_eq!(next_index(len - 1, len), len - 1);

        assert_eq!(prev_index(5), 4);

        assert_eq!(prev_index(0), 0);
    }
}
