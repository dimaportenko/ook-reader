use std::path::PathBuf;
use std::rc::Rc;

use base64::{engine::general_purpose::STANDARD, Engine};
use dioxus::desktop::{use_asset_handler, wry::http::Response};
use percent_encoding::NON_ALPHANUMERIC;
use rbook::epub::rewrite::{EpubRewriteOptions, PathRewrite};
use rbook::Epub;

use crate::web::assets::INJECTED_ASSETS;

pub(crate) const EPUB_ROUTE: &str = "epub";
pub(crate) const EPUB_URL_PREFIX: &str = "dioxus://index.html/epub/"; // must embed EPUB_ROUTE

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpineDoc {
    pub(crate) href: String,
    pub(crate) xhtml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkTarget {
    pub(crate) spine_index: usize,
    pub(crate) fragment: Option<String>,
}

pub(crate) fn extension_for(media_type: &str) -> Option<&'static str> {
    match media_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/svg+xml" => Some("svg"),
        _ => None,
    }
}

pub(crate) fn content_type_for(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "css" => "text/css",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "xhtml" | "htm" | "html" => "application/xhtml+xml",
        _ => "application/octet-stream",
    }
}

pub(crate) fn to_xhtml_data_url(xhtml: &str) -> String {
    format!(
        "data:application/xhtml+xml;base64,{}",
        STANDARD.encode(xhtml)
    )
}

pub(crate) fn load_spine(epub: &Epub) -> Result<Vec<SpineDoc>, Box<dyn std::error::Error>> {
    let rewrite = EpubRewriteOptions::default().rewrite_paths(PathRewrite::prefix(EPUB_URL_PREFIX));

    epub.reader()
        .map(|entry| {
            let entry = entry?;
            let manifest_entry = entry.manifest_entry();

            let href = manifest_entry
                .href()
                .decode()
                .trim_start_matches('/')
                .to_string();
            let xhtml = manifest_entry.read_str_with(&rewrite)?;

            Ok(SpineDoc { href, xhtml })
        })
        .collect()
}

pub(crate) fn resolve_internal_link(
    docs: &[SpineDoc],
    current_index: usize,
    href: &str,
) -> Option<LinkTarget> {
    let (path, fragment) = match href.split_once('#') {
        Some((path, frag)) => (
            path,
            Some(
                percent_encoding::percent_decode_str(frag)
                    .decode_utf8_lossy()
                    .into_owned(),
            ),
        ),
        None => (href, None),
    };

    if path.is_empty() {
        return Some(LinkTarget {
            spine_index: current_index,
            fragment,
        });
    }

    let prefix = EPUB_URL_PREFIX;
    let zip_path = path.strip_prefix(prefix)?;

    let zip_path = percent_encoding::percent_decode_str(zip_path).decode_utf8_lossy();

    let spine_index = docs.iter().position(|doc| doc.href == zip_path)?;

    Some(LinkTarget {
        spine_index,
        fragment,
    })
}

pub(crate) fn insert_before_head_close(xhtml: &str, snippet: &str) -> String {
    xhtml.replacen("</head>", &format!("{snippet}</head>"), 1)
}

pub(crate) fn render_document_url(doc: &SpineDoc, fragment: Option<&str>) -> String {
    let with_assets = insert_before_head_close(&doc.xhtml, INJECTED_ASSETS);

    let url = to_xhtml_data_url(&with_assets);
    match fragment {
        Some(frag) => format!(
            "{}#{}",
            url,
            percent_encoding::utf8_percent_encode(frag, NON_ALPHANUMERIC)
        ),
        None => url,
    }
}

fn sanitized_file_name(input: &str) -> Option<String> {
    let file_name = std::path::Path::new(input).file_name()?.to_str()?;
    (file_name == input).then(|| input.to_owned())
}

pub(crate) fn use_register_covers_handler(books_dir: PathBuf) {
    use_asset_handler("covers", move |request, responder| {
        let name = request.uri().path().rsplit('/').next().unwrap_or_default();
        let Some(name) = sanitized_file_name(name) else {
            let not_found = Response::builder()
                .status(404)
                .body(Vec::new())
                .expect("empty 404 body is always valid");
            responder.respond(not_found);
            return;
        };

        match std::fs::read(books_dir.join(&name)) {
            Ok(bytes) => {
                let body = Response::builder()
                    .header("Content-Type", content_type_for(&name))
                    .body(bytes)
                    .expect("response with a valid content-type header");
                responder.respond(body);
            }
            Err(_) => {
                let not_found = Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .expect("empty 404 body is always valid");
                responder.respond(not_found);
            }
        }
    });
}

pub(crate) fn use_register_asset_handler(epub: Rc<Epub>) {
    use_asset_handler(EPUB_ROUTE, move |request, responder| {
        let path = request
            .uri()
            .path()
            .strip_prefix(&format!("/{}", EPUB_ROUTE))
            .unwrap_or_default();

        match epub.read_resource_bytes(path) {
            Ok(bytes) => {
                let body = Response::builder()
                    .header("Content-Type", content_type_for(path))
                    .body(bytes)
                    .expect("response with a valid content-type header");
                responder.respond(body);
            }
            Err(_) => {
                let not_found = Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .expect("empty 404 body is always valid");
                responder.respond(not_found);
            }
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CoverImage {
    pub(crate) media_type: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BookMeta {
    pub(crate) title: String,
    pub(crate) author: Option<String>,
    pub(crate) cover: Option<CoverImage>,
}

pub(crate) fn read_metadata(epub: &Epub) -> Result<BookMeta, Box<dyn std::error::Error>> {
    let metadata = epub.metadata();

    let title = metadata
        .title()
        .map(|t| t.value().to_string())
        .unwrap_or_else(|| "Untitled".to_string());

    let author = metadata.creators().next().map(|c| c.value().to_string());

    let cover = epub.manifest().cover_image().and_then(|entry| {
        let bytes = entry.read_bytes().ok()?;
        Some(CoverImage {
            media_type: entry.media_type().to_string(),
            bytes,
        })
    });

    Ok(BookMeta {
        title,
        author,
        cover,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn url_prefix_embeds_the_route() {
        assert!(
            EPUB_URL_PREFIX.contains(&format!("/{EPUB_ROUTE}/")),
            "EPUB_URL_PREFIX ({EPUB_URL_PREFIX}) must contain the /{EPUB_ROUTE}/ segment",
        );
    }

    #[test]
    fn insert_before_head_close_is_a_noop_without_a_head() {
        let out = insert_before_head_close("<html><body>x</body></html>", "<style/>");
        assert_eq!(out, "<html><body>x</body></html>");
    }

    #[test]
    fn sample_epub_fixture_is_bundled() {
        let path = std::path::Path::new(crate::BOOK);
        assert!(
            path.exists(),
            "sample EPUB fixture missing at {BOOK} — is book/ gitignored or the file moved?",
            BOOK = crate::BOOK,
        );
        // Non-trivial size = a real book, not a stray empty placeholder.
        let bytes = std::fs::metadata(path).expect("fixture metadata").len();
        assert!(bytes > 100_000, "fixture looks too small ({bytes} bytes)");
    }

    #[test]
    fn loads_spine_in_reading_order() {
        let epub = Rc::new(Epub::open(crate::BOOK).expect("open fixture book"));
        let docs = load_spine(&epub).expect("should open the bundled epub");
        assert_eq!(docs.len(), 15);

        assert!(
            docs.iter()
                .any(|d| d.xhtml.contains("A Scandal in Bohemia")),
            "expected the first story's text somewhere in the spine",
        );

        assert!(
            !docs[0].xhtml.contains("A Scandal in Bohemia"),
            "index 0 should be the cover, not story one"
        );
    }

    #[test]
    fn wraps_xhtml_as_a_base64_data_url() {
        let url = to_xhtml_data_url("<html />");
        assert!(url.starts_with("data:application/xhtml+xml;base64,"));

        use base64::{engine::general_purpose::STANDARD, Engine};
        let payload = url
            .strip_prefix("data:application/xhtml+xml;base64,")
            .unwrap();
        assert_eq!(STANDARD.decode(payload).unwrap(), b"<html />");
    }

    #[test]
    fn reads_cover_image_bytes() {
        let epub = Epub::open(crate::BOOK).expect("should open the bundled epub");

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
    fn injects_pagination_css_before_head_close() {
        let xhtml = r#"<html xmlns="http://wwww.w3.org/1999/xhtml"><head><title>T</title></head><body><p>Hello</p></body></html>"#;

        let paged = insert_before_head_close(xhtml, INJECTED_ASSETS);

        assert!(paged.contains("--ook-page: 0"));
        assert!(paged.contains("column-width: calc(100vw"));
        assert!(paged.find("--ook-page: 0").unwrap() < paged.find("</head>").unwrap());
        assert!(paged.contains("<p>Hello</p>"));
    }

    #[test]
    fn ignores_external_links() {
        let epub = Rc::new(Epub::open(crate::BOOK).expect("open fixture book"));
        let docs = load_spine(&epub).expect("should open the bundled epub");

        assert_eq!(
            resolve_internal_link(&docs, 1, "https://www.gutenberg.org"),
            None
        );
    }

    #[test]
    fn resolves_contents_link_to_doc_and_fragment() {
        let epub = Rc::new(Epub::open(crate::BOOK).expect("open fixture book"));
        let docs = load_spine(&epub).expect("should open the bundled epub");

        let target = resolve_internal_link(
            &docs,
            1,
            "dioxus://index.html/epub/OEBPS/5186027266282590649_1661-h-1.htm.xhtml#chap01",
        )
        .expect("contents link should point at another spine item");

        assert_eq!(target.spine_index, 2);
        assert_eq!(target.fragment.as_deref(), Some("chap01"));
    }

    #[test]
    fn injects_page_count_probe_before_head_close() {
        let xhtml = r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body><p>Hi</p></body></html>"#;

        let out = insert_before_head_close(xhtml, INJECTED_ASSETS);

        // reports back over the bridge under its own message kind …
        assert!(out.contains("ook-pages"));
        // … derives the count from the laid-out width vs the viewport …
        assert!(out.contains("scrollWidth"));
        assert!(out.contains("innerWidth"));
        // … is injected into the head so it parses before the body it measures …
        assert!(out.find("ook-pages").unwrap() < out.find("</head>").unwrap());
        // … and leaves the original document intact.
        assert!(out.contains("<p>Hi</p>"));
    }

    #[test]
    fn reads_title_and_author_from_metadata() {
        let epub = Rc::new(Epub::open(crate::BOOK).expect("open fixture book"));
        let meta = read_metadata(&epub).expect("bundled epub metadata should read");

        assert!(
            meta.title.contains("Sherlock Holmes"),
            "expected the book's title, got {:#?}",
            meta.title,
        );

        assert!(
            meta.author.as_deref().unwrap_or("").contains("Doyle"),
            "expected Conan Doyle as the author, got {:#?}",
            meta.author,
        );
    }

    #[test]
    fn injects_page_listener_before_head_close() {
        let xhtml = r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body><p>Hi</p></body></html>"#;

        let out = insert_before_head_close(xhtml, INJECTED_ASSETS);

        assert!(out.contains("ook-set-page"));
        assert!(out.contains(r#"setProperty("--ook-page""#));
        assert!(out.find("ook-set-page").unwrap() < out.find("</head>").unwrap());
        assert!(out.contains("<p>Hi</p>"));
    }

    #[test]
    fn read_metadata_extracts_the_cover_image() {
        let epub = Epub::open(crate::BOOK).expect("open fixture book");
        let meta = read_metadata(&epub).expect("bundled epub metadata should read");

        let cover = meta.cover.expect("the bundled book declares a cover image");
        assert!(cover.media_type.starts_with("image/"));
        // Real image bytes, not a stray placeholder: JPEG → FF D8 FF, PNG → 89 50 4E 47.
        let is_jpeg = cover.bytes.starts_with(&[0xFF, 0xD8, 0xFF]);
        let is_png = cover.bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]);
        assert!(
            is_jpeg || is_png,
            "expected JPEG or PNG bytes, got {} bytes",
            cover.bytes.len()
        );
    }

    #[test]
    fn covers_route_only_serves_bare_file_names() {
        assert_eq!(
            sanitized_file_name("abc.cover.jpg"),
            Some("abc.cover.jpg".to_string())
        );
        assert_eq!(sanitized_file_name("../library.sqlite3"), None);
        assert_eq!(sanitized_file_name("a/b.jpg"), None);
        assert_eq!(sanitized_file_name(""), None);
    }

    #[test]
    fn fragment_rides_in_the_url_hash_not_the_document() {
        let doc = SpineDoc {
            href: "c1.xhtml".into(),
            xhtml: r#"<html><head></head><body><p id="chap01">Hi</p></body></html>"#.into(),
        };

        let with = render_document_url(&doc, Some("chap01"));
        let without = render_document_url(&doc, None);

        assert_eq!(with, format!("{without}#chap01"));
    }

    #[test]
    fn resolves_a_percent_encoded_href_to_a_decoded_target() {
        let docs = vec![
            SpineDoc {
                href: "OEBPS/cover.xhtml".into(),
                xhtml: String::new(),
            },
            SpineDoc {
                href: "OEBPS/Chapter 1.xhtml".into(),
                xhtml: String::new(),
            },
        ];

        let target = resolve_internal_link(
            &docs,
            0,
            &format!("{EPUB_URL_PREFIX}OEBPS/Chapter%201.xhtml#s%20a"),
        )
        .expect("an encoded href should still match its decoded spine entry");

        assert_eq!(target.spine_index, 1);
        assert_eq!(target.fragment.as_deref(), Some("s a"));
    }

    #[test]
    fn resolves_a_bare_fragment_against_the_current_chapter() {
        let docs = vec![SpineDoc {
            href: "OEBPS/c1.xhtml".into(),
            xhtml: String::new(),
        }];

        let target = resolve_internal_link(&docs, 0, "#note%201").expect("bare fragments resolve");

        assert_eq!(target.spine_index, 0);
        assert_eq!(target.fragment.as_deref(), Some("note 1"));
    }

    #[test]
    fn fragment_scroll_asset_reacts_to_hash_changes() {
        assert!(INJECTED_ASSETS.contains("hashchange"));
        assert!(INJECTED_ASSETS.contains("location.hash"));
        assert!(INJECTED_ASSETS.contains("ook-scroll"));
    }
}
