#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::desktop::{use_asset_handler, wry::http::Response};
use dioxus::prelude::*;
use rbook::epub::rewrite::{EpubRewriteOptions, PathRewrite};
use rbook::Epub;

const FAVICON: Asset = asset!("/assets/favicon.ico");
// const MAIN_CSS: Asset = asset!("/assets/main.css");
const BOOK: &str = "book/The Adventures of Sherlock Holmes by Arthur Conan Doyle.epub";

fn main() {
    dioxus::launch(App);
}

fn content_type_for(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "css" => "text/css",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

#[component]
fn App() -> Element {
    let epub = use_hook(|| Rc::new(Epub::open(BOOK).expect("should open the bundled epub")));

    use_asset_handler("epub", move |request, responder| {
        let path = request
            .uri()
            .path()
            .strip_prefix("/epub")
            .unwrap_or_default();

        match epub.read_resource_bytes(path) {
            Ok(bytes) => {
                let body = Response::builder()
                    .header("Content-Type", content_type_for(path))
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
        // document::Link {
        //     rel: "stylesheet",
        //     href: MAIN_CSS,
        // }
        SpineList {}
    }
}

#[component]
fn SpineList() -> Element {
    let docs = use_hook(|| load_spine(BOOK).expect("bundled epub should load"));
    let mut current = use_signal(|| 0usize);
    let len = docs.len();
    let current_doc = &docs[current()];

    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100vh;",
            div {
                style: "display: flex; gap: 8px;",
                button {
                    onclick: move |_| current.set(prev_index(current())),
                    "Prev"
                }

                button {
                    onclick: move |_| current.set(next_index(current(), len)),
                    "Next"
                }
            }

            iframe {
                "sandbox": "allow-same-origin",
                style: "flex: 1; width: 100%; border: none;",
                srcdoc: "{current_doc}",
            }
        }
    }
}

fn load_spine(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let epub = Epub::open(path)?;

    let rewrite = EpubRewriteOptions::default().rewrite_paths(PathRewrite::prefix("/epub/"));

    let mut docs = Vec::new();
    for entry in epub.reader() {
        let data = entry?;
        docs.push(data.manifest_entry().read_str_with(&rewrite)?);
    }

    Ok(docs)
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
    fn loads_spine_in_reading_order() {
        let docs = load_spine(BOOK).expect("should open the bundled epub");
        assert_eq!(docs.len(), 15);

        assert!(
            docs.iter().any(|d| d.contains("A Scandal in Bohemia")),
            "expected the first story's text somewhere in the spine",
        );

        assert!(
            !docs[0].contains("A Scandal in Bohemia"),
            "index 0 should be the cover, not story one"
        );
    }

    #[test]
    fn reads_cover_image_bytes() {
        let epub = Epub::open(BOOK).expect("should open the bundled epub");

        let cover = epub
            .manifest()
            .cover_image()
            .expect("this book declares a cover image");

        let bytes = cover
            .read_bytes()
            .expect("should read a cover bytes out of the zip");

        // Assert on the *bytes*, not just that it's Ok: a real image starts with a known magic
        // number. JPEG → FF D8 FF; PNG → 89 50 4E 47. If neither, you didn't get image data.
        let is_jpeg = bytes.starts_with(&[0xFF, 0xD8, 0xFF]);
        let is_png = bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]);
        assert!(
            is_jpeg || is_png,
            "cover should be a JPEG or PNG, got {} bytes",
            bytes.len()
        );

        assert!(
            cover.media_type().starts_with("image/"),
            "cover media-type should be an image/* type"
        );
    }

    #[test]
    fn rewrites_resource_paths_to_the_epub_handler() {
        let docs = load_spine(BOOK).expect("should open the bundled epub");
        assert!(
            docs.iter().any(|d| d.contains("/epub/")),
            "expected rewritten resource URLs under /epub/"
        );

        let cover_doc = &docs[0];
        assert!(
            cover_doc.contains("/epub"),
            "expected rewritten resource URLs under /epub/"
        );

        assert!(
            !cover_doc.contains("../"),
            "no unresolved OPF-relative paths should remain in the cover document"
        );
    }

    #[test]
    fn paging_clamps_at_both_ends() {
        let len = 15;

        assert_eq!(next_index(0, len), 1);

        assert_eq!(next_index(len - 1, len), len - 1);

        assert_eq!(prev_index(5), 4);

        assert_eq!(prev_index(0), 0);
    }
}
