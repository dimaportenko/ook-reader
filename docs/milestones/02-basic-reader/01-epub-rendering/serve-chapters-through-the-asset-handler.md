# Serve chapters through the asset handler

[← EPUB Rendering](README.md) · chapter-transport rework, 2 of 3

Drafted: 2026-07-25. Revised: 2026-07-26.

**Status: steps 1, 2 and 4 shipped. Step 3 landed and blanked the reader — Dioxus desktop
cancels every `dioxus://` iframe navigation after the app's own page load. Step 3b is the
replacement transport and is the open work.**

Implements the structural half of finding #1
in [`performance-review-2026-07.md`](../performance-review-2026-07.md) (and, as a consequence, finding #2).

Today a chapter reaches the iframe as a base64 `data:` URL built in Rust and handed to
RSX as an attribute value. This plan moves the chapter onto the EPUB asset handler that
already serves the book's images and CSS, so the iframe `src` becomes a short URL and
the chapter bytes never touch the VDOM.

Line references are current as of `c12ae18`.

## Where we are

```
open_epub (src/ui/library.rs:186)
  Epub::open  →  load_spine (src/epub.rs:57)          ← reads + path-rewrites EVERY chapter
  OpenBook { epub: Rc<Epub>, docs: Rc<Vec<SpineDoc>> } ← whole book resident as Strings

Reader (src/ui/reader.rs:45)
  iframe_src memo → render_document_url (src/epub.rs:118)
      insert_before_head_close(xhtml, INJECTED_ASSETS)   ← full copy
      to_xhtml_data_url  →  base64                       ← full copy, +33%
  rsx! { iframe { src: "{iframe_src}" } }                ← ~700 KB attribute through diff + IPC

use_register_asset_handler (src/epub.rs:168)
  GET /epub/OEBPS/images/1.jpg → read_resource_bytes     ← already exactly the right shape
```

The handler on the last two lines is the piece that already works. Chapters are the only
resource type that *doesn't* use it.

## Where we're going

```
open_epub
  Epub::open  →  spine_hrefs                            ← manifest only, no decompression
  OpenBook { epub: Rc<Epub>, docs: Rc<Vec<String>> }     ← ~15 short strings

Reader
  iframe_src memo → chapter_url(href, fragment)
      "dioxus://index.html/epub/OEBPS/ch1.xhtml#chap01"  ← ~50 bytes through diff + IPC

use_register_asset_handler
  GET /epub/OEBPS/ch1.xhtml
      → read_resource_str_with(rewrite)                  ← one chapter, on demand
      → insert_before_head_close(…, INJECTED_ASSETS)
      → 200 application/xhtml+xml
  GET /epub/OEBPS/images/1.jpg → read_resource_bytes     ← unchanged
```

What disappears: `to_xhtml_data_url`, the `base64` dependency, the megabyte attribute,
and the eager whole-book read in the click handler.

The `Reader` half of that diagram is what step 3 got wrong — the webview refuses to
*navigate* an iframe to a `dioxus://` URL. Step 3b keeps every other line of it and
replaces those two with a JS `fetch` and a blob URL. The handler half is unchanged and
shipped.

## The one API that makes this cheap

`rbook` already exposes the serve-time shape we need — a *resource-scoped* read with the
same rewrite options `load_spine` passes to the reader today:

```rust
// rbook-0.7.9, src/epub.rs:688
pub fn read_resource_str_with<'a>(
    &self,
    resource: impl Into<Resource<'a>>,
    rewrite: &EpubRewriteOptions,
) -> EbookResult<String>
```

Relative paths (`/OEBPS/ch1.xhtml`) are normalized from the zip root, so the path the
handler already extracts from the URI works unchanged, and percent-decoding is done by
rbook (`transform_resource` → `uri::decode`). Nothing new has to be taught about paths.

`epub.spine()` gives the reading order without reading any content — each
`EpubSpineEntry` has `.manifest_entry()` → `.href()`. That's what makes step 4 possible.

## Decisions taken up front

**Documents are detected by content type, not by a marker in the URL.** The handler
injects the assets when `content_type_for(path)` says `application/xhtml+xml`, and
streams raw bytes otherwise. One rule, one source of truth, no query parameter to keep
in sync with the links inside the book.

**The rewrite stays on, for now.** `PathRewrite::prefix(EPUB_URL_PREFIX)` keeps the
`href` attributes inside the document absolute, which is what `resolve_internal_link`
(`src/epub.rs:77`) parses. Serving from `/epub/...` means relative paths would *also*
resolve correctly on their own — that's step 5, deliberately separated so the transport
change and the link-resolution change don't land in the same commit.

**`Cache-Control: no-store` on everything under `/epub`.** Chapter URLs are
book-agnostic: `/epub/OEBPS/ch1.xhtml` names a different document in every book, and the
handler is re-registered per `Reader` mount with a different `Rc<Epub>`. A cached
response from the previously open book would be served for the newly opened one. This is
a **pre-existing** hazard for images (same paths, same handler, today), which this change
extends to chapter documents — so it gets fixed here.

> Alternative considered: put the book id in the path
> (`/epub/{book_id}/OEBPS/ch1.xhtml`) and let the webview cache normally. It's the more
> correct answer and it preserves caching, but it makes `EPUB_URL_PREFIX` a runtime value
> that has to be threaded into the rewrite options, the src builder and
> `resolve_internal_link`. Not worth it until caching is measurably wanted.

**The handler body becomes a pure function.** `use_asset_handler` runs inside a webview;
nothing about it is reachable from `cargo test`. Extracting

```rust
pub(crate) fn serve_epub_resource(epub: &Epub, path: &str) -> Option<Served>
```

puts every serve-time decision — injection, content type, charset, 404 — under unit test
against the bundled fixture, and leaves the hook as a five-line adapter that builds a
`Response`. This is the single most valuable structural move in the plan and it comes
first, before any behavior changes.

## The steps

Each step is independently committable and leaves the app working.

### Step 1 — extract the pure serve function (no behavior change)

The check first — this is the test that only becomes possible after the extraction, and
it passes on the *current* behavior:

```rust
#[test]
fn serves_an_image_resource_as_raw_bytes() {
    let epub = Epub::open(crate::BOOK).expect("open fixture book");

    let served = serve_epub_resource(&epub, "/OEBPS/images/cover.jpg")
        .expect("the fixture's cover is reachable by path");

    assert!(served.content_type.starts_with("image/"));
    assert!(served.body.starts_with(&[0xFF, 0xD8, 0xFF]) || served.body.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
}

#[test]
fn serving_an_unknown_path_is_a_miss() {
    let epub = Epub::open(crate::BOOK).expect("open fixture book");
    assert!(serve_epub_resource(&epub, "/OEBPS/nope.xhtml").is_none());
}
```

(Confirm the fixture's actual image path first — `epub.manifest()` iteration in a
throwaway `dbg!` is enough, or reuse `manifest().cover_image()`.)

Then the implementation sketch:

```rust
pub(crate) struct Served {
    pub(crate) content_type: &'static str,
    pub(crate) body: Vec<u8>,
}

pub(crate) fn serve_epub_resource(epub: &Epub, path: &str) -> Option<Served> {
    let body = epub.read_resource_bytes(path).ok()?;
    Some(Served { content_type: content_type_for(path), body })
}
```

and `use_register_asset_handler` shrinks to: strip the `/epub` prefix, call
`serve_epub_resource`, map `Some`/`None` to a 200/404 `Response`. Same bytes on the wire
as before.

### Step 2 — inject the assets at serve time

Still invisible at runtime: the iframe is on data URLs, and the only things fetched
through the handler are images and CSS, which this step doesn't touch. The whole step is
verified by tests.

The check first:

```rust
#[test]
fn serving_a_chapter_injects_the_reader_assets() {
    let epub = Epub::open(crate::BOOK).expect("open fixture book");
    let href = spine_hrefs(&epub).expect("fixture spine")[2].clone();  // step 4 note: use load_spine()[2].href for now

    let served = serve_epub_resource(&epub, &format!("/{href}"))
        .expect("a spine document is reachable by its href");

    let xhtml = String::from_utf8(served.body).expect("chapters are utf-8");

    assert!(xhtml.contains("--ook-page: 0"));                    // pagination.css
    assert!(xhtml.contains("ook-pages"));                        // page-count.js
    assert!(xhtml.contains("hashchange"));                       // fragment-scroll.js
    assert!(xhtml.find("--ook-page: 0").unwrap() < xhtml.find("</head>").unwrap());
    assert!(xhtml.contains("A Scandal in Bohemia"));             // the chapter survived
    assert!(served.content_type.starts_with("application/xhtml+xml"));
}

#[test]
fn serving_a_chapter_rewrites_resource_paths_to_the_epub_route() {
    // whatever the fixture's chapter 0 (the cover page) references, it must come back
    // pointing at EPUB_URL_PREFIX rather than a relative path.
}
```

Implementation sketch — the branch is the whole step:

```rust
pub(crate) fn serve_epub_resource(epub: &Epub, path: &str) -> Option<Served> {
    let content_type = content_type_for(path);

    if content_type == XHTML {
        let rewrite = EpubRewriteOptions::default()
            .rewrite_paths(PathRewrite::prefix(EPUB_URL_PREFIX));
        let xhtml = epub.read_resource_str_with(path, &rewrite).ok()?;
        let with_assets = insert_before_head_close(&xhtml, INJECTED_ASSETS);
        return Some(Served { content_type: XHTML_UTF8, body: with_assets.into_bytes() });
    }

    let body = epub.read_resource_bytes(path).ok()?;
    Some(Served { content_type, body })
}
```

Two details that are easy to miss:

- **`; charset=utf-8` on the document content type.** `read_resource_str_with` hands back
  a Rust `String`, so what we serve is always UTF-8 — but the chapter may carry an XML
  prolog declaring something else (`encoding="ISO-8859-1"`). For XML, the HTTP charset
  parameter wins over the prolog, so stating it is what keeps such a book from
  mojibake-ing. The `data:` URL never had this problem because it was base64 of the
  original bytes' UTF-8 re-encoding either way; the handler needs it stated explicitly.
- **`content_type_for` maps `htm`/`html` to `application/xhtml+xml` too**
  (`src/epub.rs:45`). That's what happens today via the data URL, so keep it — but know
  that it means strict XML parsing: a malformed `.htm` chapter renders as an XML parse
  error, not tag soup. Changing it to `text/html` is a separate, behavior-changing
  decision; don't fold it in here.

Also add the `Cache-Control: no-store` header in the hook while you're in it (it's a
header on the `Response`, so it lives in the adapter, not in `Served`).

### Step 3 — point the iframe at the URL

This is the step where the win lands. The check first, replacing
`fragment_rides_in_the_url_hash_not_the_document`, which still asserts the exact property
the fragment design rests on — the document part of the URL is independent of the
fragment:

```rust
#[test]
fn the_chapter_url_is_short_and_carries_the_fragment_in_the_hash() {
    let with = chapter_url("OEBPS/c1.xhtml", Some("chap01"));
    let without = chapter_url("OEBPS/c1.xhtml", None);

    assert_eq!(without, "dioxus://index.html/epub/OEBPS/c1.xhtml");
    assert_eq!(with, format!("{without}#chap01"));
}

#[test]
fn the_chapter_url_encodes_spaces_but_keeps_path_separators() {
    let url = chapter_url("OEBPS/Chapter 1.xhtml", Some("s a"));
    assert_eq!(url, "dioxus://index.html/epub/OEBPS/Chapter%201.xhtml#s%20a");
}
```

The second test is the one that earns its keep. `render_document_url` currently encodes
the fragment with `NON_ALPHANUMERIC` (`src/epub.rs:126`), which is right for a fragment
and *wrong* for a path — it would encode every `/`. The path needs its own set:

```rust
use percent_encoding::{AsciiSet, CONTROLS};

// URL-unsafe characters, minus '/' which must survive as the path separator.
// '#' and '?' matter most: an unencoded one would truncate the path.
const PATH: &AsciiSet = &CONTROLS
    .add(b' ').add(b'"').add(b'<').add(b'>').add(b'`')
    .add(b'#').add(b'?').add(b'%').add(b'{').add(b'}');

pub(crate) fn chapter_url(href: &str, fragment: Option<&str>) -> String {
    let path = utf8_percent_encode(href, PATH);
    match fragment {
        Some(frag) => format!(
            "{EPUB_URL_PREFIX}{path}#{}",
            utf8_percent_encode(frag, NON_ALPHANUMERIC)
        ),
        None => format!("{EPUB_URL_PREFIX}{path}"),
    }
}
```

`load_spine` stores the *decoded* href (`.decode()`, `src/epub.rs:67`), so this encoding
step is mandatory, not defensive — the fixture already contains hrefs that survive only
because base64 hid them.

Then in `Reader` (`src/ui/reader.rs:45`):

```rust
let iframe_src = use_memo(move || {
    let doc = &docs_for_iframe[chapter()];
    epub::chapter_url(&doc.href, pending_fragment().as_deref())
});
```

Deletions in the same commit: `render_document_url`, `to_xhtml_data_url`, its test
`wraps_xhtml_as_a_base64_data_url`, the `base64` import, and `base64 = "0.22"` from
`Cargo.toml` (grep confirms `src/epub.rs` is its only user).

`insert_before_head_close` and the four `injects_*_before_head_close` tests stay —
injection moved, it didn't go away.

**This is the first step with runtime risk**, so it carries a manual checklist (below).

> **This step does not work and cannot be made to work on Dioxus desktop 0.7.9.** It
> landed, the reader went blank, and the transport half of it is superseded by step 3b
> below. Everything else in step 3 — deleting `render_document_url`, `to_xhtml_data_url`
> and the `base64` dependency — stands.

### Step 3b — load the chapter from JavaScript into a blob URL

**This is the open work.**

#### What happened

Steps 1, 2 and 4 landed clean — `cargo build` warning-free, 36 tests green. Step 3 landed
and the reader went blank. The `iframe` carried the right `src`
(`dioxus://index.html/epub/OEBPS/wrap0000.xhtml`), but its `#document` was
`<html><head></head><body></body></html>` with no DOCTYPE — the shape of `about:blank` —
and the label read `Page 1 of 0`, meaning no injected script ever ran. Nothing appeared in
the Network panel.

The Rust side was never at fault. Serving that exact path outside the app returns the
document with the assets in it:

```
--- /OEBPS/wrap0000.xhtml
  OK  614 bytes, has </head>: true
```

#### Why

`dioxus-desktop-0.7.9`, `src/webview.rs:370`:

```rust
.with_navigation_handler(move |var| {
    // Serve the index and assets.
    if var.starts_with("dioxus://") || var.starts_with("http://dioxus.") || … {
        // After the page has loaded once, don't allow any more navigation
        let page_loaded = page_loaded.swap(true, Ordering::SeqCst);
        return !page_loaded;
    }
    …
    // By default, external links are allowed. This keeps things like iframes working.
    navigation_handler.as_ref().map(|f| f(&var)).unwrap_or(true)
})
```

The policy is one-shot. The app's own load of `dioxus://index.html/` flips `page_loaded`
to `true`; every later `dioxus://` navigation returns `false`, and wry turns that into
`WKNavigationActionPolicy::Cancel` (`wry-0.53.5`, `src/wkwebview/navigation.rs:75`). The
iframe's navigation is cancelled *before any request is made* — hence no 404, no error
page, no network entry, just a frame still sitting on `about:blank`.

The distinction that makes this hard to see:

- **Subresources** — the images and CSS a chapter references — never touch the navigation
  policy. They always worked through the asset handler, and still do. That is why step 2's
  tests pass and why the handler looked healthy.
- **Navigations** are what the policy gates. The old `data:` URL fell through to the last
  branch and was allowed — note Dioxus's own comment there: *"This keeps things like
  iframes working"*. The moment step 3 turned the chapter into a URL, it became a
  navigation, and the policy killed it.

Two escape hatches that turn out not to exist: `cfg.navigation_handler`, the app-supplied
one, is consulted **only** in that final branch, so application code cannot re-allow
`dioxus://` navigation; and 0.7.9 is the last 0.7 release, so no upgrade changes this.

#### Alternatives considered

- **Serve the book on its own scheme.** `Config::with_custom_protocol` (`config.rs:226`)
  is bound only by `'static` — no `Send`/`Sync` — so an `Rc<Epub>` can live in such a
  handler, and an `ookbook://…` URL misses all three branches and is allowed. The plan
  would survive literally. Rejected on two counts: the protocol is registered at launch,
  so the handler leaves the component tree and "which book is open" becomes shared mutable
  state outside `Reader`; and on Windows wry maps custom schemes to
  `http://ookbook.localhost/`, which hits the `webbrowser::open(&var); return false`
  branch — the chapter would open in the user's browser. A worse milestone-04 landmine
  than the one step 5 was meant to retire.
- **Revert to the data URL, keep step 4.** Read each chapter lazily in the memo and base64
  it there. Keeps finding #2, gives up finding #1. This is the fallback if the step below
  fails its check.

#### The decision: fetch in JS, hand the iframe a `blob:` URL

The asset handler *is* reachable by `fetch` — that's a subresource load, not a navigation.
So the parent fetches the chapter itself and gives the iframe a URL that isn't `dioxus://`:

```
Reader effect  →  eval(chapter-loader.js).send((url, fragment))    ← ~50 bytes
chapter-loader.js
  fetch("dioxus://index.html/epub/OEBPS/ch1.xhtml")                ← subresource: allowed
  → asset handler → serve_epub_resource → 200 + INJECTED_ASSETS
  → response.blob()
  → URL.createObjectURL(blob) + "#chap01"
  → iframe.src = blob:…                                            ← not dioxus://: allowed
```

**`blob:`, not `srcdoc`.** Both dodge the policy, but a blob URL is a real URL with a real
hash, so `hashchange` and the whole design in
[`fragment-scroll-via-url-hash.md`](fragment-scroll-via-url-hash.md) survive untouched. A
`srcdoc` document's URL is `about:srcdoc`, and setting `location.hash` on it is
engine-dependent — not something to bet the fragment design on. A blob also inherits the
creating document's origin, so the frame stays same-origin with the app and both
`postMessage` directions keep working.

**`response.blob()`, not `response.text()`.** `.text()` decodes the chapter's UTF-8 into a
UTF-16 JavaScript string, and `new Blob([text])` re-encodes it back — two full copies of
the chapter for nothing. `.blob()` takes the bytes as they are *and* carries
`application/xhtml+xml; charset=utf-8` through from the response header, so the Blob's
type is right without restating it.

**Revoke the previous blob URL.** This is the line that decides whether the change was
worth making. Miss it and every chapter turn leaks ~59 KB inside the webview, so after one
read-through you are holding the whole book again — precisely the resident-memory profile
step 4 just deleted.

**Fetch on chapter change; move the hash on fragment change.** Re-fetching a 56 KB chapter
merely to scroll to a different anchor inside it would be worse than what step 3 replaced.
Only a chapter change should fetch; a fragment change within the chapter already loaded
sets `frame.contentWindow.location.hash` (same-origin, so the parent may) and lets
`fragment-scroll.js` react. This is the real reason to fix `on_scroll`'s unconditional
`pending_fragment` write (step 6) — under this transport it is a correctness point, not a
cosmetic one.

#### The check first

No Rust needed, and it validates the entire approach before anything is written. In the
reader's devtools console:

```js
const f = document.getElementById("reader-frame");
const r = await fetch("dioxus://index.html/epub/OEBPS/wrap0000.xhtml");
console.log(r.status, r.headers.get("content-type"));
f.src = URL.createObjectURL(await r.blob());
```

Expected: `200 application/xhtml+xml; charset=utf-8`, and the cover renders **paginated** —
the assets are already in those bytes. Then repeat against a real chapter with `+
"#chap01"` appended, to confirm the fragment lands. If this fails, stop and take the data
URL revert above; everything past here depends on it.

#### Then the implementation

The `iframe` loses every reactive attribute, so it stops taking part in the diff at all:

```rust
iframe {
    id: "reader-frame",
    "sandbox": "allow-same-origin allow-scripts",
    style: "flex: 1; width: 100%; border: none;",
}
```

`iframe_src` (`src/ui/reader.rs:45`) is deleted and replaced by an effect. Values travel
through `Eval::send` — `fn send(&self, data: impl Serialize) -> Result<(), EvalError>`
(`dioxus-document-0.7.9`, `src/eval.rs:31`) — rather than being interpolated into the
script source, which is the same injection hygiene R6(a) was about:

```rust
use_effect(move || {
    let url = epub::chapter_url(&docs_for_iframe[chapter()]);
    let fragment = pending_fragment();
    let loader = document::eval(CHAPTER_LOADER_JS);
    _ = loader.send((url, fragment));
});
```

and `src/web/assets/chapter-loader.js`:

```js
const [url, fragment] = await dioxus.recv();
const frame = document.getElementById("reader-frame");

const response = await fetch(url);
if (!response.ok) return;

// .blob(), not .text(): no UTF-16 round trip, and the response's
// Content-Type rides along as the Blob's own type.
const blob = await response.blob();

// Without this the webview keeps every chapter you have visited.
if (frame.dataset.blobUrl) URL.revokeObjectURL(frame.dataset.blobUrl);

const next = URL.createObjectURL(blob);
frame.dataset.blobUrl = next;
frame.src = fragment ? `${next}#${encodeURIComponent(fragment)}` : next;
```

Ripples:

- `chapter_url` loses its `fragment` parameter: the hash goes on the *blob* URL, in
  JavaScript, so the Rust-side `NON_ALPHANUMERIC` encoding goes with it.
  `the_chapter_url_encodes_spaces_but_keeps_path_separators` keeps all of its value (the
  `PATH` set still guards the fetch URL);
  `the_chapter_url_is_short_and_carries_the_fragment_in_the_hash` loses its fragment half,
  and that assertion moves onto the manual checklist.
- If `send` turns out to race the script's first `recv`, hoist the loader into a
  long-lived `use_future` that loops on `recv` — the shape `use_bridge`
  (`src/ui/reader.rs:137`) already uses — and have the effect send into that instead.
- The `ook-set-page` effect (`src/ui/reader.rs:53`) now races an async *fetch* on top of
  the navigation. The fix is the one already on the checklist: post on the iframe's `load`.
- Nothing in `src/epub.rs` changes. `serve_epub_resource`, `epub_response`, the handler,
  `spine_hrefs` and `INJECTED_ASSETS` are all correct as shipped.

#### Why the performance case still holds

Measured on the bundled fixture: 633,506 bytes of XHTML across the spine, largest chapter
56,899 bytes, `INJECTED_ASSETS` 2,050 bytes.

| | Before | Step 3 as planned | Step 3b |
| --- | --- | --- | --- |
| At open | 619 KB read, rewritten, resident for the session | manifest only | manifest only |
| Per chapter, through the VDOM diff + IPC | ~78 KB base64 attribute | ~50 B `src` attribute | nothing |
| Into the webview | 78 KB (base64, +33%) | 59 KB raw | 59 KB raw |
| Resident after 15 chapter turns | 619 KB + last string | 0 | 0 *if revoked* |

Step 3b costs one extra async hop and holds the chapter in a Blob for a moment instead of
streaming it into the parser — a constant of about 59 KB. In one respect it beats the
planned step 3: with no reactive attribute left on the `iframe`, chapter *and* fragment
changes stop re-rendering that node at all.

Revisiting a chapter re-fetches and re-decompresses it, exactly as `no-store` would have
forced anyway. If that ever shows up, an LRU of blob URLs keyed by href is now an easy win
because the JS side owns the fetch — a better caching story than the one `no-store`
deliberately gave up.

### Step 4 — stop reading the whole book at open time

Finding #2. With the handler reading chapters on demand, `SpineDoc.xhtml` has no
remaining reader.

```rust
#[test]
fn spine_hrefs_are_read_without_decompressing_chapters() {
    let epub = Epub::open(crate::BOOK).expect("open fixture book");
    let hrefs = spine_hrefs(&epub).expect("fixture spine");

    assert_eq!(hrefs.len(), 15);                       // same count load_spine produced
    assert!(hrefs[2].ends_with(".xhtml"));
    assert!(hrefs.iter().all(|h| !h.starts_with('/'))); // relative to the zip root, as before
}
```

The count assertion is the one to watch: `load_spine` iterates `epub.reader()`, and
`spine_hrefs` would iterate `epub.spine()`. Those agree by default, but `EpubReaderOptions`
has a `linear_behavior` knob — if the count comes back different, that's why, and the
answer is to match the reader's behavior rather than to change the assertion.

```rust
pub(crate) fn spine_hrefs(epub: &Epub) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    epub.spine()
        .into_iter()
        .map(|entry| {
            let manifest_entry = entry.manifest_entry().ok_or("spine entry with a dangling idref")?;
            Ok(manifest_entry.href().decode().trim_start_matches('/').to_string())
        })
        .collect()
}
```

`manifest_entry()` returns `Option` — resist `filter_map`. Silently dropping an entry
shifts every later spine index, and chapter indices are what the whole navigation layer
and the persisted reading position are keyed on. Failing loudly at open time preserves
the "Reader is infallible once it has a book" trade the original design made.

Ripples:

- `SpineDoc` loses `xhtml` and becomes redundant with `String` — collapse it, or keep the
  newtype if you prefer the name at call sites. `resolve_internal_link` takes `&[String]`.
- `OpenBook.docs: Rc<Vec<String>>` (`src/ui/library.rs:18`), `use_bridge` signature
  (`src/ui/reader.rs:137`), `open_epub` (`src/ui/library.rs:186`).
- `loads_spine_in_reading_order` (`src/epub.rs:265`) asserts on chapter *text*; that
  assertion migrates to the step-2 serve test, which is where the text now comes from.
- The three `resolve_internal_link` tests build `SpineDoc { href, xhtml: String::new() }`
  literals — they get shorter, and their intent is unchanged.

After this step `open_epub` is a zip open plus a manifest parse, and the click handler no
longer holds the UI thread for the length of a book.

### Step 5 — ~~optional: drop the path rewrite and go platform-relative~~ off the table

**Closed by step 3b.** This step rested on the document being served from the same URL
space as its resources. Under a blob URL it is not: a relative `../images/1.png` would
resolve against `blob:dioxus://…/<uuid>` and land nowhere. `PathRewrite::prefix` is now
load-bearing permanently, and `link-bridge.js` keeps posting `a.getAttribute("href")`
because the rewrite guarantees that attribute is absolute.

The portability point it was going to retire still needs an answer eventually: the
`dioxus://index.html/` origin is hardcoded in `EPUB_URL_PREFIX`, and the fetch URL could
be made root-relative (`/epub/OEBPS/ch1.xhtml`) on its own, the way `BookCover` already
does with `src: "/covers/{name}"` (`src/ui/library.rs:111`). That is a smaller, separable
change — it is only the *rewritten* paths inside the document that must stay absolute.

The original reasoning is kept below as a record.

Two things become possible once documents are served from the same URL space as their
resources, and they're worth doing together:

- A relative `../images/1.png` inside a chapter now resolves *natively* against the
  document's own URL, landing on `/epub/OEBPS/images/1.png` — the exact path the rewrite
  was synthesizing. `EpubRewriteOptions` can go back to default, which removes a full
  content rewrite from every chapter serve.
- With the rewrite gone, nothing needs the hardcoded `dioxus://index.html/` origin in the
  document. Making the iframe `src` root-relative (`/epub/OEBPS/ch1.xhtml`) makes it
  resolve against whatever origin the platform's webview actually uses — `dioxus://` on
  macOS, `http://dioxus.localhost/` on Windows. `BookCover` already does exactly this
  with `src: "/covers/{name}"` (`src/ui/library.rs:111`), so the pattern is proven in
  this app. That retires a milestone-04 portability landmine.

The catch: `link-bridge.js` posts `a.getAttribute("href")` — the **raw** attribute. With
the rewrite gone that's a relative path, and `resolve_internal_link` has no base to
resolve it against. The fix is one word: post `a.href`, which is the browser's *resolved*
absolute URL, and is stable regardless of what the rewrite does. Then
`resolve_internal_link` strips everything up to and including `/epub/` rather than a
fixed origin string.

Consequence to expect: a bare `#frag` link arrives as the full document URL plus hash, so
the `path.is_empty()` early-return branch (`src/epub.rs:94`) stops firing — the spine
lookup finds the current chapter by href and produces the same `LinkTarget`. Keep the
branch (it costs nothing and `resolves_a_bare_fragment_against_the_current_chapter`
documents the intent), but know it's no longer the live path.

### Step 6 — optional: the leftovers from the review

Both are one-liners that the earlier steps make trivially safe:

- `on_scroll` writes `pending_fragment` unconditionally (`src/nav.rs:95`), re-running the
  `iframe_src` memo. ~~After step 3 the memo is a `format!` of two short strings, so this
  is no longer a performance point — but guarding the write (`if it's Some`) is still the
  correct expression of the intent.~~ **Promoted by step 3b: not optional and not
  cosmetic.** Under the blob transport the loader effect reads `pending_fragment`, so an
  unguarded write re-runs it — and a naive loader would re-fetch and re-blob the whole
  chapter to clear a hash. Note the guard as originally written (`if it's Some`) does
  *not* help: the fragment *is* `Some` at that moment, having just been navigated to. The
  loader has to distinguish "chapter changed" (fetch) from "fragment changed" (set
  `location.hash` on the document already loaded).
- `resolve_internal_link`'s O(spine) scan (`src/epub.rs:106`) becomes a `HashMap` built
  once beside the href list, if you want it. At 15 entries it is not a real cost.

## What can't be unit-tested — the `dx serve` checklist

Everything past step 3 needs eyes on the running app, because the transport is the part
tests can't reach. Step 3 proved how badly: it shipped with a green suite and a blank
reader, because the failure was a navigation policy two crates away. Nothing below is
optional.

- Open a book → the first chapter renders, **with pagination applied**. A page that
  renders as an unstyled scrolling wall means `INJECTED_ASSETS` didn't make it in; a
  page that renders as an XML error tree means the content type or the charset is wrong.
- Turn pages within a chapter, then across a chapter boundary in both directions.
  Backwards across a boundary is the one that exercises `pending_last` → `pages:` →
  `page = N-1`, which depends on the `load` event firing after the *new* transport
  commits.
- Open Contents, click a chapter → lands on the right page. Then click a **different
  anchor in the same chapter** → it moves without a reload. That's the `hashchange` path
  from [`fragment-scroll-via-url-hash.md`](fragment-scroll-via-url-hash.md); step 3b must
  not break it, and it's the assertion no test covers. Watch the Network panel while you
  do it: a second fetch of the same chapter means the loader is treating a fragment change
  as a chapter change.
- **Blob URLs are revoked.** Page through five or six chapters, then check that the
  webview isn't accumulating them — the frame should only ever hold one live
  `blob:` URL in `dataset.blobUrl`, and memory should not climb by a chapter per turn.
  This is the one step-3b failure that looks like success.
- A chapter with images → images still load. This is the one that proves the rewrite (or,
  after step 5, native relative resolution) still points at a servable path.
- **Close the book and open a *different* one.** If chapter text or a cover image from the
  first book shows up, `Cache-Control: no-store` isn't taking effect and the per-book path
  segment is needed after all.
- Watch for a one-frame flicker on chapter change: the `ook-set-page` effect
  (`src/ui/reader.rs:53`) posts into `iframe.contentWindow`, which during an async
  navigation is still the *outgoing* document. Cosmetic if it appears, and the fix is to
  post on the iframe's `load` rather than on the page signal.

## Risks

| Risk | Signal | Response |
| --- | --- | --- |
| Stale response across books | Wrong chapter/cover after switching books | `no-store` (step 2); escalate to `/epub/{book_id}/…` paths |
| A missing resource is now a silent blank frame, not an open-time error | Empty iframe, no message | Serve a minimal XHTML error document (with `INJECTED_ASSETS`, so paging still works) for 404s on document requests |
| Handler runs synchronously on the UI thread | Hitch on chapter change in a huge book | It's one zip entry, not a book — but `RequestAsyncResponder` is `Send`, so the read can move off-thread later if it ever shows up |
| Strict XML parsing on a malformed `.htm` chapter | XML parse error page | Pre-existing (the data URL declared the same type); revisit `content_type_for` separately |
| `epub.spine()` and `epub.reader()` disagree on entry count | Step 4's `len() == 15` assertion fails | Match the reader's `linear_behavior`; do not weaken the assertion |
| **Blob URLs never revoked** | Memory climbs by ~59 KB per chapter turn; the whole book resident after one read-through | `revokeObjectURL` before each swap (step 3b); it silently undoes finding #2 |
| **A fragment change re-fetches the chapter** | Second network entry for the same chapter when clicking an anchor within it | The loader must branch on chapter-changed vs fragment-changed (step 3b / step 6) |
| A framework upgrade changes the navigation policy | — | If Dioxus ever allows `dioxus://` iframe navigation, step 3 becomes viable again and step 3b's JS layer can be deleted; nothing in `src/epub.rs` would change |

## Leftovers from steps 1, 2 and 4

Found reviewing what shipped. None of them blocks step 3b; all of them should land before
this doc is closed.

- `base64 = "0.22"` is still in `Cargo.toml:12`. The import went, the dependency didn't.
- **`epub_response` has no test.** Extracting it was what made `Cache-Control` and the 404
  status testable, and neither is asserted anywhere — nothing would catch the header being
  dropped. `wry::http::Response` needs no webview to build, so this is a plain unit test.
- The charset still isn't pinned: `starts_with("application/xhtml+xml")` passes for bare
  `XHTML` too, so the one detail step 2 called out as easy to miss is the one detail no
  test covers. `assert_eq!(served.content_type, XHTML_UTF8)` passes today.
- A document 404 carries no `Content-Type` — an empty, MIME-less response that renders as
  a blank frame with no error. That's the second row of the risks table, still open, and
  it is exactly the symptom step 3's navigation failure produced, so it is worth closing
  purely to keep the two failure modes distinguishable.
- `src/epub.rs:495` still has a debug `println!("{:?}", href)`, and both serve tests carry
  stale `// step 4: becomes spine_hrefs(&epub)[2]` comments plus a wrong assert message
  ("expected the cover wrapper at spine index 0" now guards a chapter at index 2).
- `src/epub.rs:56` recomputes `content_type_for(path)` when `content_type` is already
  bound at `:41`.

Worth noting where the test gap was: the app opens on spine index 0, `wrap0000.xhtml`, and
both serve tests were moved off index 0 onto index 2. The one document the reader shows
first was the one document no test exercised. It happened to be innocent here — the
failure was the navigation policy — but that is luck, not coverage.

## Definition of done

- No chapter bytes cross the VDOM at all; the `iframe` has no reactive attributes. No
  `base64` in `Cargo.toml`.
- Opening a book performs no chapter decompression; `OpenBook` holds hrefs, not text.
- Every serve-time decision is covered by a test against the bundled fixture through
  `serve_epub_resource`, and `epub_response`'s headers and 404 are covered too.
- Blob URLs are revoked on every swap.
- The `dx serve` checklist passes, including the two-clicks-in-the-same-chapter case and
  the blob-accumulation check.
- [`performance-review-2026-07.md`](../performance-review-2026-07.md) findings #1 and #2 are marked resolved,
  with the "As shipped" deltas recorded at the bottom of this file — same as
  [`fragment-scroll-via-url-hash.md`](fragment-scroll-via-url-hash.md) does.
