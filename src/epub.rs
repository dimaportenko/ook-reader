use base64::{engine::general_purpose::STANDARD, Engine};
use rbook::epub::rewrite::{EpubRewriteOptions, PathRewrite};
use rbook::Epub;

pub(crate) const EPUB_ROUTE: &str = "epub";

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

pub(crate) fn load_spine(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let epub = Epub::open(path)?;

    let rewrite = EpubRewriteOptions::default().rewrite_paths(PathRewrite::prefix(format!(
        "dioxus://index.html/{EPUB_ROUTE}/"
    )));

    epub.reader()
        .map(|entry| Ok(entry?.manifest_entry().read_str_with(&rewrite)?))
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
            padding: 0 !important;
            box-sizing: border-box !important;
            max-width: none !important;

            overflow: visible;
            column-width: 100vw;
            column-gap: 0 !important;
            column-fill: auto;
            transform: translateX(calc(var(--ook-page) * -100vw));
        }}
        </style>"#,
    );

    xhtml.replacen("</head>", &format!("{css}</head>"), 1)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn loads_spine_in_reading_order() {
        let docs = load_spine(crate::BOOK).expect("should open the bundled epub");
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
        assert!(paged.contains("column-width: 100vw"));
        assert!(paged.find("--ook-page: 2").unwrap() < paged.find("</head>").unwrap());
        assert!(paged.contains("<p>Hello</p>"));
    }
}
