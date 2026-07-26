# Performance review (July 2026)

[← Milestone 2](README.md) · sibling of
[`review-2026-07-steps.md`](review-2026-07-steps.md)

Reviewed: 2026-07-24. Scope: all of `src/` (`main.rs`, `epub.rs`, `library.rs`,
`nav.rs`, `ui/reader.rs`, `ui/library.rs`) plus `Cargo.toml`.

Updated 2026-07-25 after `2a2b5f8` (consolidated injected assets) and `30e4b0c`
(fragment via URL hash): finding #1's cheap fix and the fragment-reload point are
resolved. Line references below are current as of `30e4b0c`.

## Verdict

For the current scale — a desktop app, one book open at a time, a handful of
library entries — nothing here is slow in practice. The findings below are about
how the design *scales* with book size, and most of them trace back to one
architectural decision: **chapters travel to the iframe as base64 data URLs.**
Adopting the epub asset handler you already built as the transport for chapters
would resolve findings 1, 2, and the fragment-reload point in one move.

Findings are ranked by impact.

## 1. The chapter render pipeline copies the whole chapter, then base64-inflates it

`src/epub.rs:118` (`render_document_url`) — **cheap fix done in `2a2b5f8`; structural
fix still open**

Originally `render_document_url` chained five injection steps, each calling
`insert_before_head_close` (`replacen` + `format!`) — a brand-new copy of the entire
chapter XHTML per call. `2a2b5f8` collapsed all five snippets into a single
compile-time `INJECTED_ASSETS` const (`src/web/assets.rs`), so there is now **one**
`replacen`. `30e4b0c` removed the last conditional injection by moving the fragment
into the URL hash, which keeps it at one.

What remains: `to_xhtml_data_url` base64-encodes the result (another full copy, 33%
larger), and that megabyte-scale string becomes an *attribute value* in RSX
(`src/ui/reader.rs:84`). So it gets diffed by the VDOM and shipped across the
Dioxus-to-webview IPC channel on every chapter switch. For a 500 KB chapter that's
now roughly 1.5 MB of transient allocation (down from 3–4 MB) plus a ~700 KB
attribute string through the diff/IPC path, per navigation.

- **Structural (the real win, still open):** you already have an asset handler
  serving the epub at `dioxus://index.html/epub/...` (`src/epub.rs:168`). If the
  iframe `src` pointed at that route and the handler injected the assets at serve
  time, the data URL, the base64 step, and the giant attribute all disappear — the
  src becomes a short URL and the chapter bytes flow through the protocol handler
  instead of the VDOM. This also unlocks finding #2. Implementation plan:
  [`serve-chapters-through-the-asset-handler.md`](01-epub-rendering/serve-chapters-through-the-asset-handler.md).

## 2. Opening a book loads and rewrites the entire spine synchronously in an onclick

`src/ui/library.rs:186` (`open_epub`), `src/epub.rs:57` (`load_spine`)

`open_epub` → `load_spine` decompresses and path-rewrites every chapter of the
book inside the click handler, on the UI thread. Per the git history this was a
deliberate trade ("load spine at open time so Reader is infallible"), and for
typical novels it's fine. But cost scales linearly with book size: a large EPUB
means a visible UI freeze on click, and the whole rewritten book stays resident in
`Rc<Vec<SpineDoc>>` for as long as it's open.

If you adopt the serve-through-the-asset-handler approach from #1, you'd only need
the spine *hrefs* up front (cheap), reading each chapter lazily when it's requested
— keeping most of the infallibility while dropping both the latency and the memory
footprint. Short of that, wrapping the load in `spawn` with a loading state would
at least unblock the UI.

## 3. Import is fully synchronous on the UI thread

`src/ui/library.rs:141` (`ImportControl` onchange)

`add_from_path` does `canonicalize` + `fs::copy` of the whole EPUB + `Epub::open`
+ cover extraction + cover write + a DB write, all inside the `onchange` handler.
A big file on a slow disk freezes the window with no feedback. Same medicine as #2:
move it into an async task and set `status` when it finishes. (The cleanup/rollback
logic itself is nicely done.)

## Minor points

- ~~**Fragment links reload the whole chapter**~~ — **fixed in `30e4b0c`.** The
  fragment now rides in the data URL's hash instead of changing the document body, so
  a same-chapter TOC jump (and the `pending_fragment` → `None` write right after it)
  is a *same-document* navigation: no reparse, no re-fired page-count probe. Design
  and rationale: [`fragment-scroll-via-url-hash.md`](01-epub-rendering/fragment-scroll-via-url-hash.md).
- **`on_scroll` writes `pending_fragment` unconditionally** (`src/nav.rs:95`):
  setting it to `None` when it's already `None` still marks the store dirty, which
  re-runs the `iframe_src` memo (`src/ui/reader.rs:43`) — i.e., the base64 half of
  finding #1. The iframe itself no longer reloads (same document, hash-only change),
  but the recompute is still wasted; guarding the write (only clear when it's `Some`)
  avoids it.
- **`books()` clones the whole `Vec<Book>` every render** (`src/ui/library.rs:38`),
  and each row clones the `Book` again for `BookCover`. Iterating over
  `books.read().iter()` borrows instead. Harmless at ten books; it's the idiom
  worth learning.
- **`resolve_internal_link` is an O(spine) linear scan per click**
  (`src/epub.rs:106`) — genuinely fine at spine sizes (~15 here), just noting a
  `HashMap<href, index>` built once would be the scalable form.
- **No `[profile.release]` in `Cargo.toml`**: when you get to shipping builds,
  `lto = true` and `codegen-units = 1` typically shave binary size and give a few
  percent runtime for free.

## What not to change

- The page-turn mechanism itself (CSS `--ook-page` transform driven by
  postMessage) is genuinely fast — no reflow-heavy scrolling, no reload per page.
- The SQLite usage (RETURNING clauses, one-shot queries) is appropriately simple
  for the scale.
