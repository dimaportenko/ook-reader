#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::desktop::{use_asset_handler, wry::http::Response};
use dioxus::prelude::*;
use rbook::Epub;

mod epub;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
pub(crate) const BOOK: &str = "book/The Adventures of Sherlock Holmes by Arthur Conan Doyle.epub";

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
    let len = docs.len();
    let current_doc = &docs[current()];

    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100vh;",

            iframe {
                "sandbox": "allow-same-origin",
                style: "flex: 1; width: 100%; border: none;",
                src: "{epub::to_xhtml_data_url(current_doc)}",
            }

            div {
                style: "display: flex; gap: 8px; padding: 8px; justify-content: center;",
                button {
                    onclick: move |_| current.set(prev_index(current())),
                    "Prev"
                }

                span {
                    "{current()}"
                }

                button {
                    onclick: move |_| current.set(next_index(current(), len)),
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
