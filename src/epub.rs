use std::path::PathBuf;
use std::rc::Rc;

use dioxus::desktop::{use_asset_handler, wry::http::Response};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use rbook::epub::rewrite::{EpubRewriteOptions, PathRewrite};
use rbook::Epub;

use crate::web::assets::{wrap_css_str, INJECTED_ASSETS, READING_SYSTEM_DEFAULTS};
use crate::web::settings::Settings;

pub(crate) const EPUB_ROUTE: &str = "epub";
pub(crate) const EPUB_URL_PREFIX: &str = "dioxus://index.html/epub/"; // must embed EPUB_ROUTE

const XHTML: &str = "application/xhtml+xml";
const XHTML_UTF8: &str = "application/xhtml+xml; charset=utf-8";

const PATH: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'#')
    .add(b'?')
    .add(b'%')
    .add(b'{')
    .add(b'}');

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("spine entry with a dangling idref")]
    DanglingIdref,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkTarget {
    pub(crate) spine_index: usize,
    pub(crate) fragment: Option<String>,
}

pub(crate) struct Served {
    pub(crate) content_type: String,
    pub(crate) body: Vec<u8>,
}

pub(crate) fn serve_epub_resource(epub: &Epub, path: &str, settings: Settings) -> Option<Served> {
    let content_type = epub
        .manifest()
        .by_href(path.trim_start_matches('/'))
        .map(|entry| entry.media_type().to_owned())
        .unwrap_or_else(|| content_type_for(path).to_owned());

    if content_type == XHTML || content_type == "text/html" {
        let rewrite =
            EpubRewriteOptions::default().rewrite_paths(PathRewrite::prefix(EPUB_URL_PREFIX));
        let xhtml = epub.read_resource_str_with(path, &rewrite).ok()?;

        let inject_css = format!("{INJECTED_ASSETS}{}", wrap_css_str(&settings.user_layer()));

        let with_defaults = insert_after_head_open(&xhtml, READING_SYSTEM_DEFAULTS);
        let with_assets = insert_before_head_close(&with_defaults, &inject_css);
        return Some(Served {
            content_type: XHTML_UTF8.to_owned(),
            body: with_assets.into_bytes(),
        });
    }

    let body = epub.read_resource_bytes(path).ok()?;
    Some(Served { content_type, body })
}

pub(crate) fn epub_response(served: Option<Served>) -> Response<Vec<u8>> {
    let builder = Response::builder().header("Cache-Control", "no-store");

    match served {
        Some(served) => builder
            .header("Content-Type", served.content_type)
            .body(served.body)
            .expect("response with valid content type header"),
        None => builder
            .status(404)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(Vec::new())
            .expect("404 always valid response"),
    }
}

pub(crate) fn chapter_url(href: &str) -> String {
    format!("{EPUB_URL_PREFIX}{}", utf8_percent_encode(href, PATH))
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
        "xhtml" | "htm" | "html" | "xml" => XHTML,
        _ => "application/octet-stream",
    }
}

pub(crate) fn resolve_internal_link(
    hrefs: &[String],
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

    let spine_index = hrefs.iter().position(|href| *href == zip_path)?;

    Some(LinkTarget {
        spine_index,
        fragment,
    })
}

pub(crate) fn insert_before_head_close(xhtml: &str, snippet: &str) -> String {
    xhtml.replacen("</head>", &format!("{snippet}</head>"), 1)
}

pub(crate) fn insert_after_head_open(xhtml: &str, snippet: &str) -> String {
    let Some(start) = xhtml.find("<head") else {
        return xhtml.to_owned();
    };

    let rest = &xhtml[start + "<head".len()..];
    if !rest.starts_with('>') && !rest.starts_with(char::is_whitespace) {
        return xhtml.to_owned();
    }

    let Some(end) = rest.find('>') else {
        return xhtml.to_owned();
    };

    let at = start + "<head".len() + end + 1;
    format!("{}{snippet}{}", &xhtml[..at], &xhtml[at..])
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

fn zip_path_for(uri_path: &str) -> String {
    let path = uri_path
        .strip_prefix(&format!("/{EPUB_ROUTE}"))
        .unwrap_or_default();

    percent_encoding::percent_decode_str(path)
        .decode_utf8_lossy()
        .into_owned()
}

pub(crate) fn use_register_asset_handler(epub: Rc<Epub>, settings: Settings) {
    use_asset_handler(EPUB_ROUTE, move |request, responder| {
        let path = zip_path_for(request.uri().path());

        responder.respond(epub_response(serve_epub_resource(&epub, &path, settings)));
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

pub(crate) fn read_metadata(epub: &Epub) -> BookMeta {
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

    BookMeta {
        title,
        author,
        cover,
    }
}

pub(crate) fn spine_hrefs(epub: &Epub) -> Result<Vec<String>, Error> {
    epub.spine()
        .into_iter()
        .map(|entry| {
            let manifest_entry = entry.manifest_entry().ok_or(Error::DanglingIdref)?;
            Ok(manifest_entry
                .href()
                .decode()
                .trim_start_matches('/')
                .to_string())
        })
        .collect()
}

#[cfg(test)]
mod test {
    use crate::web::theme::Theme;

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
        let path = std::path::Path::new(crate::TEST_BOOK);
        assert!(
            path.exists(),
            "sample EPUB fixture missing at {TEST_BOOK} — is book/ gitignored or the file moved?",
            TEST_BOOK = crate::TEST_BOOK,
        );
        // Non-trivial size = a real book, not a stray empty placeholder.
        let bytes = std::fs::metadata(path).expect("fixture metadata").len();
        assert!(bytes > 100_000, "fixture looks too small ({bytes} bytes)");
    }

    #[test]
    fn reads_cover_image_bytes() {
        let epub = Epub::open(crate::TEST_BOOK).expect("should open the bundled epub");

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
        let epub = Rc::new(Epub::open(crate::TEST_BOOK).expect("open fixture book"));
        let docs = spine_hrefs(&epub).expect("should open the bundled epub");

        assert_eq!(
            resolve_internal_link(&docs, 1, "https://www.gutenberg.org"),
            None
        );
    }

    #[test]
    fn resolves_contents_link_to_doc_and_fragment() {
        let epub = Rc::new(Epub::open(crate::TEST_BOOK).expect("open fixture book"));
        let docs = spine_hrefs(&epub).expect("should open the bundled epub");

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
        let epub = Rc::new(Epub::open(crate::TEST_BOOK).expect("open fixture book"));
        let meta = read_metadata(&epub);

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
    fn injects_a_theme_listener_before_head_close() {
        let xhtml = r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body><p>Hi</p></body></html>"#;

        let out = insert_before_head_close(xhtml, INJECTED_ASSETS);

        assert!(out.contains("ook-set-theme"));
        assert!(out.find("ook-set-theme").unwrap() < out.find("</head>").unwrap());
        assert!(out.contains("<p>Hi</p>"));
    }

    #[test]
    fn read_metadata_extracts_the_cover_image() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");
        let meta = read_metadata(&epub);

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
    fn resolves_a_percent_encoded_href_to_a_decoded_target() {
        let docs = vec!["OEBPS/cover.xhtml".into(), "OEBPS/Chapter 1.xhtml".into()];

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
        let docs = vec!["OEBPS/c1.xhtml".into()];

        let target = resolve_internal_link(&docs, 0, "#note%201").expect("bare fragments resolve");

        assert_eq!(target.spine_index, 0);
        assert_eq!(target.fragment.as_deref(), Some("note 1"));
    }

    #[test]
    fn fragment_scroll_asset_reacts_to_hash_changes() {
        assert!(INJECTED_ASSETS.contains("hashchange"));
        assert!(INJECTED_ASSETS.contains("location.hash"));
        assert!(INJECTED_ASSETS.contains("ook-scroll"));
        // A fragment naming an id the document does not have must still post a
        // page, because that message is what clears `Pending::Fragment`. Staying
        // silent leaves it to be re-applied to the next chapter.
        assert!(INJECTED_ASSETS.contains("currentPage"));
    }

    #[test]
    fn serves_an_image_resource_as_raw_bytes() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");

        let served = serve_epub_resource(
            &epub,
            "/OEBPS/374963762688302552_cover.jpg",
            Settings::default(),
        )
        .expect("the fixture's cover is reachable by path");

        assert!(served.content_type.starts_with("image/"));
        assert!(
            served.body.starts_with(&[0xFF, 0xD8, 0xFF])
                || served.body.starts_with(&[0x89, 0x50, 0x4E, 0x47])
        );
    }

    #[test]
    fn serving_an_unknown_path_is_a_miss() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");
        assert!(serve_epub_resource(&epub, "/OEBPS/nope.xhtml", Settings::default()).is_none());
    }

    #[test]
    fn serving_a_chapter_injects_the_reader_assets() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");
        let hrefs = spine_hrefs(&epub).expect("fixture spine");

        let href = hrefs.get(2).expect("3d item in spine exists");
        let served = serve_epub_resource(&epub, &format!("/{href}"), Settings::default())
            .expect("a spine document is reachable by its href");

        let xhtml = String::from_utf8(served.body).expect("chapters are utf-8");

        assert!(xhtml.contains("--ook-page: 0")); // pagination.css
        assert!(xhtml.contains("ook-pages")); // page-count.js
        assert!(xhtml.contains("hashchange")); // fragment-scroll.js
        assert!(xhtml.contains("ook-position")); // page-position.js
        assert!(xhtml.contains("ook-sel:")); // fragment-scroll.js
        assert!(xhtml.find("--ook-page: 0").unwrap() < xhtml.find("</head>").unwrap());
        assert!(xhtml.contains("A SCANDAL IN BOHEMIA")); // the chapter survived
        assert!(served.content_type.starts_with("application/xhtml+xml"));
    }

    #[test]
    fn serving_a_chapter_injects_the_theme_after_every_other_layer() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");
        let hrefs = spine_hrefs(&epub).expect("fixture spine");

        let href = hrefs.get(2).expect("3d item in spine exists");
        let served = serve_epub_resource(
            &epub,
            &format!("/{href}"),
            Settings {
                theme: Theme::Night,
            },
        )
        .expect("a spine document is reachable by its href");

        let xhtml = String::from_utf8(served.body).expect("chapters are utf-8");
        let (background, text) = Theme::Night.colors();

        assert!(
            xhtml.contains(&format!("--USER__backgroundColor: {background}")),
            "the chosen theme never reached the served document",
        );
        assert!(xhtml.contains(&format!("--USER__textColor: {text}")));

        let author = xhtml.find("pgepub.css").expect("the book's own stylesheet");
        let rs = xhtml.find("--ook-page: 0").expect("pagination.css");
        let user = xhtml
            .find("--USER__backgroundColor")
            .expect("the theme layer");

        assert!(
            user > author,
            "the USER layer must come after the book's CSS to win at equal specificity",
        );
        assert!(
            user > rs,
            "the USER layer must come after pagination.css to win at equal !important",
        );
        assert!(user < xhtml.find("</head>").expect("a head to close"));
    }

    #[test]
    fn serving_a_chapter_rewrites_resource_paths_to_the_epub_route() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");
        let hrefs = spine_hrefs(&epub).expect("fixture spine");

        let href = hrefs.get(2).expect("3d item in spine exists");
        assert!(
            href.ends_with("5186027266282590649_1661-h-1.htm.xhtml"),
            "expected the first story at spine index 2, got {href}",
        );

        let served = serve_epub_resource(&epub, &format!("/{href}"), Settings::default())
            .expect("a spine document is reachable by its href");
        let xhtml = String::from_utf8(served.body).expect("chapters are utf-8");

        let css = format!("{EPUB_URL_PREFIX}OEBPS/pgepub.css");
        let cover = format!("{EPUB_URL_PREFIX}OEBPS/374963762688302552_cover.jpg");

        assert!(
            xhtml.contains(&css),
            "stylesheet link was not rewritten to {css}:\n{xhtml}",
        );
        assert!(
            xhtml.contains(&cover),
            "svg cover image was not rewritten to {cover}:\n{xhtml}",
        );

        assert!(!xhtml.contains(r#"href="pgepub.css""#));
        assert!(!xhtml.contains(r#"xlink:href="374963762688302552_cover.jpg""#));

        let path = cover
            .strip_prefix(EPUB_URL_PREFIX)
            .expect("prefixed by construction");
        let image = serve_epub_resource(&epub, &format!("/{path}"), Settings::default())
            .expect("a rewritten path must round-trip back through the handler");

        assert_eq!(image.content_type, "image/jpeg");
        assert!(image.body.starts_with(&[0xFF, 0xD8, 0xFF]));
    }

    #[test]
    fn the_chapter_url_is_the_route_plus_the_zip_path() {
        // The fragment is no longer the URL's business — chapter-loader.js appends
        // it to the blob URL, because fetch would discard a hash anyway.
        assert_eq!(
            chapter_url("OEBPS/c1.xhtml"),
            "dioxus://index.html/epub/OEBPS/c1.xhtml",
        );
    }

    #[test]
    fn the_chapter_url_encodes_spaces_but_keeps_path_separators() {
        assert_eq!(
            chapter_url("OEBPS/Chapter 1.xhtml"),
            "dioxus://index.html/epub/OEBPS/Chapter%201.xhtml",
        );
    }

    #[test]
    fn spine_hrefs_are_relative_zip_paths_in_reading_order() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");
        let hrefs = spine_hrefs(&epub).expect("fixture spine");

        assert_eq!(hrefs.len(), 15); // same count load_spine produced
        assert!(hrefs[2].ends_with(".xhtml"));
        assert!(hrefs.iter().all(|h| !h.starts_with('/'))); // relative to the zip root, as before
    }

    #[test]
    fn the_handler_decodes_percent_escapes_before_looking_up_the_zip_entry() {
        // chapter_url writes the escape; this is the matching decode. Without it a
        // book with a space in a filename 404s on every chapter.
        assert_eq!(
            zip_path_for("/epub/OEBPS/Chapter%201.xhtml"),
            "/OEBPS/Chapter 1.xhtml",
        );
        // an unescaped path survives untouched
        assert_eq!(zip_path_for("/epub/OEBPS/c1.xhtml"), "/OEBPS/c1.xhtml");
        // a path outside the route yields nothing to serve
        assert_eq!(zip_path_for("/nope"), "");
    }

    #[test]
    fn a_served_resource_is_typed_and_never_cached() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");
        let hrefs = spine_hrefs(&epub).expect("fixture spine");

        let response = epub_response(serve_epub_resource(
            &epub,
            &format!("/{}", hrefs[2]),
            Settings::default(),
        ));

        assert_eq!(response.status(), 200);
        // the charset is load-bearing: an XHTML document with no declared encoding
        // would otherwise be decoded by the webview's locale default
        assert_eq!(response.headers()["Content-Type"], XHTML_UTF8);
        // the epub is already in memory; a second copy in the webview cache buys
        // nothing and would go stale if the book were reimported
        assert_eq!(response.headers()["Cache-Control"], "no-store");
    }

    #[test]
    fn a_missing_resource_is_a_typed_404() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");

        let response = epub_response(serve_epub_resource(
            &epub,
            "/OEBPS/nope.xhtml",
            Settings::default(),
        ));

        assert_eq!(response.status(), 404);
        assert_eq!(
            response.headers()["Content-Type"],
            "text/plain; charset=utf-8"
        );
    }

    #[test]
    fn insert_after_head_open_writes_inside_a_head_that_has_attributes() {
        let xhtml =
            r#"<html><head profile="http://example.org/p"><title>T</title></head><body/></html>"#;

        let out = insert_after_head_open(xhtml, "<style/>");

        assert!(out.contains(r#"<head profile="http://example.org/p"><style/><title>T</title>"#));
    }

    #[test]
    fn insert_after_head_open_is_a_noop_without_a_head() {
        let out = insert_after_head_open("<html><body>x</body></html>", "<style/>");

        assert_eq!(out, "<html><body>x</body></html>");
    }

    #[test]
    fn insert_after_head_open_does_not_mistake_a_header_for_a_head() {
        // `<head` is a prefix of `<header>`, which is ordinary XHTML5 sectioning content.
        let xhtml = "<html><body><header>Chapter</header><p>x</p></body></html>";

        let out = insert_after_head_open(xhtml, "<style/>");

        assert_eq!(out, xhtml);
    }

    #[test]
    fn the_three_cascade_layers_are_served_in_priority_order() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");
        let hrefs = spine_hrefs(&epub).expect("fixture spine");

        let href = hrefs.get(2).expect("3d item in spine exists");
        let served = serve_epub_resource(&epub, &format!("/{href}"), Settings::default())
            .expect("a spine document is reachable by its href");
        let xhtml = String::from_utf8(served.body).expect("chapters are utf-8");

        let rs = xhtml.find("--RS__").expect("the reading-system defaults");
        let author = xhtml.find("pgepub.css").expect("the book's own stylesheet");
        let user = xhtml.find("--USER__").expect("the theme layer");

        assert!(rs < author, "RS defaults must lose to the book's CSS");
        assert!(author < user, "the book's CSS must lose to the USER layer");
    }
}
