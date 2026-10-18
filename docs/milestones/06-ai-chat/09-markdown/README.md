# Feature — Markdown replies

[← Milestone 6](../README.md)

The models answer in Markdown: `**bold**`, bullet lists, fenced code, the odd table. The
chat drawer shows that text raw, asterisks and all. This feature renders an **assistant**
reply as formatted text, both a finished one and the bubble that is still streaming. User
messages stay plain, because they carry the prefilled book passage, and Markdown would
mangle a quoted `*` or a line starting with `1.`.

| # | Phase | Status |
|---|---|---|
| 26 | [Markdown replies](phase-26-markdown.md) | 🚧 in progress — captured 2026-10-18 |

## Research

Checked on 2026-10-18:

- **Parser.** [`pulldown-cmark`](https://crates.io/crates/pulldown-cmark) 0.13.4 is the
  latest stable release. Its default features are `getopts` and `html`, and `html` only
  gates `html::push_html`. With `default-features = false` you still get the full
  `Parser`/`Event`/`Tag` pull API, and that's the only part this feature uses.
  `Parser::new_ext(src, Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES)` turns on
  the two GFM extensions in scope. The exact `Tag` fields (for example
  `Tag::Link { link_type, dest_url, title, id }`) must be checked against
  [docs.rs](https://docs.rs/pulldown-cmark/0.13.4) or `cargo doc` when a step uses them, not
  taken from memory.
- **Links already leave the app.** In dioxus-desktop 0.7.9 (locked), `src/webview.rs`
  installs a wry `with_navigation_handler`. Any navigation to `http://`, `https://` or
  `mailto:` calls `webbrowser::open(&url)` and is cancelled, so the WebView stays on the
  reader. `dioxus/mobile` uses the same crate on iOS, and `webbrowser` 1.2.1 has an iOS
  backend (`UIApplication openURL`). **That makes a plain `a { href }` correct for those
  three schemes.** Any other scheme or a relative href takes the handler's default branch,
  which *allows* the navigation, so those must not be rendered as anchors. The iOS path is
  read from source and hasn't been run on a device yet.

## Risks

- **Mid-stream Markdown.** The streaming bubble re-parses the whole text on every update,
  so an unclosed ```` ``` ```` or a lone `**` renders in whatever state it's in until the
  next tokens arrive. CommonMark treats an unclosed fence as running to the end of the
  document, which is the right look while code is streaming. A test pins that behaviour.
- **Untrusted text.** A reply is model output, so it can contain raw HTML or
  `javascript:` links. The typed tree has no HTML node. Raw HTML becomes literal text, and
  only the three safe link schemes become anchors.
- **Wide content.** Code lines and tables can be wider than the drawer, which on a phone
  is the whole screen. They scroll sideways inside the bubble and never widen the page.
