use std::rc::Rc;

use base64::{engine::general_purpose::STANDARD, Engine};
use dioxus::desktop::{use_asset_handler, wry::http::Response};
use rbook::epub::rewrite::{EpubRewriteOptions, PathRewrite};
use rbook::Epub;

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

pub(crate) fn load_spine(path: &str) -> Result<Vec<SpineDoc>, Box<dyn std::error::Error>> {
    let epub = Epub::open(path)?;

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

pub(crate) fn inject_pagination_css(xhtml: &str, page: usize) -> String {
    let css = format!(
        r#"<style type="text/css">
        :root {{ --ook-page: {page}; }}
        html {{ width: 100vw; height: 100vh; overflow: hidden; }}
        body {{
            width: 100vw !important;
            height: 100vh !important;
            margin: 0 !important;
            padding: 24px !important;
            box-sizing: border-box !important;
            max-width: none !important;

            overflow: visible;
            column-width: calc(100vw - 48px) !important;
            column-gap: 48px !important;
            column-fill: auto;
            transform: translateX(calc(var(--ook-page) * -100vw));
        }}
        </style>"#,
    );

    insert_before_head_close(xhtml, &css)
}

pub(crate) fn inject_link_bridge(xhtml: &str) -> String {
    let script = r#"<script type="text/javascript">
    //<![CDATA[
        document.addEventListener('click', function(e) {
            var a = e.target.closest && e.target.closest('a[href]');
            if (!a) return;
            e.preventDefault();
            window.parent.postMessage(
                { kind: 'ook-link', raw: a.getAttribute('href') },
                '*'
            );
        });
    //]]>
    </script>"#;

    insert_before_head_close(xhtml, script)
}

pub(crate) fn resolve_internal_link(
    docs: &[SpineDoc],
    current_index: usize,
    href: &str,
) -> Option<LinkTarget> {
    let (path, fragment) = match href.split_once('#') {
        Some((path, frag)) => (path, Some(frag.to_string())),
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

pub(crate) fn inject_fragment_scroll(xhtml: &str, fragment: &str) -> String {
    let script = format!(
        r#"<script type="text/javascript">
        //<![CDATA[
            window.addEventListener('load', function() {{
                var el = document.getElementById("{fragment}");
                if (!el) return;
                var page = Math.round(el.offsetLeft / window.innerWidth);
                window.parent.postMessage({{ kind: 'ook-scroll', page: page }}, '*');
            }});

        //]]>
        </script>"#,
    );

    insert_before_head_close(xhtml, &script)
}

pub(crate) fn insert_before_head_close(xhtml: &str, snippet: &str) -> String {
    xhtml.replacen("</head>", &format!("{snippet}</head>"), 1)
}

pub(crate) fn render_document_url(doc: &SpineDoc, page: usize, fragment: Option<&str>) -> String {
    let paged = inject_pagination_css(&doc.xhtml, page);
    let bridged = inject_link_bridge(&paged);
    let probed = inject_page_count_probe(&bridged);
    let prepared = match fragment {
        Some(frag) => inject_fragment_scroll(&probed, frag),
        None => probed,
    };
    to_xhtml_data_url(&prepared)
}

pub(crate) fn inject_page_count_probe(xhtml: &str) -> String {
    let script = r#"<script type="text/javascript">
    //<![CDATA[
        const report = function() {
            var count = Math.max(
                1,
                Math.ceil(document.body.scrollWidth / window.innerWidth)
            );
            window.parent.postMessage({ kind: 'ook-pages', count: count }, '*');
        };
        window.addEventListener('load', report);
        window.addEventListener('resize', report);
    //]]>
    </script>"#;

    insert_before_head_close(xhtml, script)
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
        let docs = load_spine(crate::BOOK).expect("should open the bundled epub");
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

        let paged = inject_pagination_css(xhtml, 2);

        assert!(paged.contains("--ook-page: 2"));
        assert!(paged.contains("column-width: calc(100vw"));
        assert!(paged.find("--ook-page: 2").unwrap() < paged.find("</head>").unwrap());
        assert!(paged.contains("<p>Hello</p>"));
    }

    #[test]
    fn ignores_external_links() {
        let docs = load_spine(crate::BOOK).expect("should open the bundled epub");

        assert_eq!(
            resolve_internal_link(&docs, 1, "https://www.gutenberg.org"),
            None
        );
    }

    #[test]
    fn resolves_contents_link_to_doc_and_fragment() {
        let docs = load_spine(crate::BOOK).expect("should open the bundled epub");

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
    fn injects_fragment_scroll_before_head_close() {
        let xhtml = r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body><p id="x">Hi</p></body></html>"#;

        let out = inject_fragment_scroll(xhtml, "chap02");

        // The script targets the requested anchor id …
        assert!(out.contains(r#"getElementById("chap02")"#));
        // … reports back over the bridge under a distinct message kind …
        assert!(out.contains("ook-scroll"));
        // … is injected into the head (so it parses before the body it measures) …
        assert!(out.find("ook-scroll").unwrap() < out.find("</head>").unwrap());
        // … and leaves the original document intact.
        assert!(out.contains(r#"<p id="x">Hi</p>"#));
    }

    #[test]
    fn injects_page_count_probe_before_head_close() {
        let xhtml = r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body><p>Hi</p></body></html>"#;

        let out = inject_page_count_probe(xhtml);

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
}
