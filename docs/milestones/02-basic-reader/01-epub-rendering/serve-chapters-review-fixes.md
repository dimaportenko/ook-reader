# Review fixes — asset-handler chapter loading

[← EPUB Rendering](README.md) · chapter-transport rework, 3 of 3

Follow-up to [`serve-chapters-through-the-asset-handler.md`](serve-chapters-through-the-asset-handler.md),
written after reviewing the shipped implementation of steps 1–4 and 3b.

Status at the time of writing: 36 tests pass, `base64` is out of `Cargo.toml`, the
duplicate `content_type_for` call is collapsed, and `"xml"` is in the extension map so
Feedbooks-style books (Hamlet) render. What follows is what the review turned up.

Two of these are behaviour bugs (§1, §2). The rest is cleanup. Each section gives a
runnable check first, then the change.

---

## 1. The loader refetches the chapter and undoes its own fragment jump

### What goes wrong

`src/ui/reader.rs:67` subscribes to **both** signals:

```rust
use_effect(move || {
    let url = epub::chapter_url(&docs_for_iframe[chapter()], None);
    let fragment = pending_fragment();          // ← reactive read
    let loader = document::eval(CHAPTER_LOADER_JS);
    _ = loader.send((url, fragment));
});
```

Click a link in the table of contents and this happens:

1. `follow_link` sets `chapter = 2`, `pending_fragment = Some("chap01")`.
2. The effect runs. `chapter-loader.js` fetches, builds a blob, sets `frame.src = blob#chap01`.
3. The document loads. `fragment-scroll.js`'s `load` handler finds `#chap01`, computes its
   page, posts `ook-scroll`.
4. `on_scroll` (`src/nav.rs:93`) sets the page **and clears `pending_fragment` to `None`**.
5. The effect sees `Some → None` and **runs again** — refetching the same chapter, building
   a second blob, and reloading the frame with no hash.

The reload lands at `--ook-page: 0`. The page-post effect at `src/ui/reader.rs:50` cannot
rescue it: `page()` did not change across the reload, so it never re-posts into the new
document. The visible result is that the link jumps to the anchor and then bounces back to
the top of the chapter, having fetched the chapter twice.

It terminates rather than looping — the second load reports `scroll:0`, which writes `None`
over `None`, and an unchanged signal does not re-run the effect.

### Why the fix goes in JavaScript, not in the effect

Three obvious Rust-side fixes each break something else:

- **`pending_fragment.peek()` in the effect** (depend on the chapter alone) removes the
  bounce, but then a link into the *current* chapter never fires the effect at all, so
  same-chapter anchors stop working. They work today.
- **Stop `on_scroll` clearing the fragment** leaves a stale fragment behind, which the next
  plain chapter step would re-apply to a fresh document.
- **Clear the fragment in `apply()` instead** hits the same wall: any write to
  `pending_fragment` is observed by the effect, so plain paging would trigger a refetch.

The tension is that the effect cannot distinguish "the chapter changed" from "only the
fragment changed" — but the frame can, because it already knows what it is displaying.
So keep the Rust side exactly as it is and let the loader decide.

### The check first

With the app running, open a book, click a TOC entry, and watch the Network panel. Today
you get **two** requests for the chapter and the view snaps back to page 1. After the fix:
one request, and the view stays on the anchor's page. Then click an anchor pointing into
the chapter you are already in — that should produce **zero** requests and still jump.

### The change — `src/web/assets/chapter-loader.js`

Replace the whole file:

```js
const [url, fragment] = await dioxus.recv();
const frame = document.getElementById("reader-frame");

// Same chapter as the one on screen? Then only the hash can have changed.
// Move it in place: a same-document navigation, so no refetch and no reload.
// `fragment-scroll.js` is already listening for `hashchange`.
if (frame.dataset.chapterUrl === url) {
  // `fragment` is null when `on_scroll` cleared it — that is not a navigation.
  if (fragment) {
    const win = frame.contentWindow;
    // Assigning the hash it already has fires no `hashchange`, so clear it
    // first; otherwise clicking the same anchor twice does nothing the second
    // time. The empty hash is inert — `reportFragmentPage` returns on no id.
    win.location.hash = "";
    win.location.hash = encodeURIComponent(fragment);
  }
  return;
}

// Claim the frame before awaiting, so a chapter change that starts while this
// fetch is in flight can tell us to stand down.
frame.dataset.pendingUrl = url;

const response = await fetch(url);
if (!response.ok) {
  console.error(`ook: ${response.status} loading ${url}`);
  return;
}

// .blob(), not .text(): no UTF-16 round trip, and the response's Content-Type
// rides along as the Blob's own type — which is what decides whether WebKit
// renders the frame or treats it as a download.
const blob = await response.blob();

// A newer load overwrote the claim while we were awaiting. Drop this one
// rather than fighting over the frame and showing the wrong chapter.
if (frame.dataset.pendingUrl !== url) return;

// Without this the webview keeps every chapter you have visited.
if (frame.dataset.blobUrl) {
  URL.revokeObjectURL(frame.dataset.blobUrl);
}

const next = URL.createObjectURL(blob);
frame.dataset.blobUrl = next;
frame.dataset.chapterUrl = url;
frame.src = fragment ? `${next}#${encodeURIComponent(fragment)}` : next;
```

Three separate problems close here: the bounce (`chapterUrl` guard), the silent blank on a
failed fetch (`console.error` — this is what would have made the Hamlet bug obvious in one
look), and the concurrent-load race described in §3.

`src/ui/reader.rs` needs no change for this, beyond the signature edit in §4.

### Left open

Two chapter changes in a row can leave the *first* chapter's blob unrevoked, because the
losing load returns before reaching the revoke. Bounded by how fast you can click, and each
blob is tens of KB. Closing the book also leaves the last blob alive — nothing revokes on
unmount. Both are worth a `use_hook_with_cleanup` later, neither is worth one now.

---

## 2. Percent-encoded URL paths never reach the right zip entry

### What goes wrong

`chapter_url` encodes with the `PATH` set (`src/epub.rs:17`), so a space in a filename
becomes `%20`. The handler then passes the path straight through:

```rust
let path = request
    .uri()
    .path()                                   // still percent-encoded
    .strip_prefix(&format!("/{}", EPUB_ROUTE))
    .unwrap_or_default();

responder.respond(epub_response(serve_epub_resource(&epub, path)));
```

`Uri::path()` does not decode. Zip entry names are literal, so rbook looks for
`Chapter%201.xhtml`, finds nothing, and the handler answers 404. The same applies to any
subresource whose rewritten href carries an escape.

`resolve_internal_link` already decodes on the link side (`src/epub.rs:116` and `:134`), so
today the two halves of the round trip disagree. Neither the Gutenberg fixture nor Hamlet
has an escape in a filename, which is why nothing catches it.

This predates the change — images went through the same handler before — but it is now on
the chapter path too, where a miss blanks the whole reader instead of dropping one image.

### The check first

Add this test; it fails before the change and passes after.

```rust
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
```

### The change — `src/epub.rs`

The decode has to be testable without a webview, so lift the path handling out of the
closure. Add next to `serve_epub_resource`:

```rust
/// Turns the URL path the webview handed us into a zip entry name.
///
/// Two things happen here. The `/epub` route prefix comes off, and the percent
/// escapes come out: `Uri::path()` hands back the encoded form, but zip entry
/// names are literal, so `Chapter%201.xhtml` would never match `Chapter 1.xhtml`.
fn zip_path_for(uri_path: &str) -> String {
    let path = uri_path
        .strip_prefix(&format!("/{EPUB_ROUTE}"))
        .unwrap_or_default();

    percent_encoding::percent_decode_str(path)
        .decode_utf8_lossy()
        .into_owned()
}
```

and collapse the handler (`src/epub.rs:184`) to:

```rust
pub(crate) fn use_register_asset_handler(epub: Rc<Epub>) {
    use_asset_handler(EPUB_ROUTE, move |request, responder| {
        let path = zip_path_for(request.uri().path());
        responder.respond(epub_response(serve_epub_resource(&epub, &path)));
    })
}
```

---

## 3. Concurrent chapter loads race on `frame.dataset.blobUrl`

Two fast chapter steps run two loader scripts concurrently. Both read
`frame.dataset.blobUrl` before either writes it, so one blob leaks; worse, the slower fetch
can resolve last and display the chapter you already navigated away from.

Fixed by the `pendingUrl` claim in §1 — no separate change.

---

## 4. `chapter_url`'s `fragment` parameter is dead

`src/ui/reader.rs:68` always passes `None`, and it cannot do otherwise: `fetch` discards the
hash, so the fragment has to be applied by the loader when it sets `frame.src`. The only
callers passing `Some` are two tests.

### The change — `src/epub.rs:72`

```rust
pub(crate) fn chapter_url(href: &str) -> String {
    format!("{EPUB_URL_PREFIX}{}", utf8_percent_encode(href, PATH))
}
```

`NON_ALPHANUMERIC` was used only for the fragment, so drop it from the import at
`src/epub.rs:5`:

```rust
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
```

### Callers

`src/ui/reader.rs:68` becomes:

```rust
let url = epub::chapter_url(&docs_for_iframe[chapter()]);
```

### Tests

`the_chapter_url_is_short_and_carries_the_fragment_in_the_hash` (`src/epub.rs:536`) is now
half about something that moved to JavaScript. Rename and shrink it:

```rust
#[test]
fn the_chapter_url_is_the_route_plus_the_zip_path() {
    // The fragment is no longer the URL's business — chapter-loader.js appends
    // it to the blob URL, because fetch would discard a hash anyway.
    assert_eq!(
        chapter_url("OEBPS/c1.xhtml"),
        "dioxus://index.html/epub/OEBPS/c1.xhtml",
    );
}
```

and `the_chapter_url_encodes_spaces_but_keeps_path_separators` (`src/epub.rs:545`) drops its
second argument:

```rust
#[test]
fn the_chapter_url_encodes_spaces_but_keeps_path_separators() {
    assert_eq!(
        chapter_url("OEBPS/Chapter 1.xhtml"),
        "dioxus://index.html/epub/OEBPS/Chapter%201.xhtml",
    );
}
```

---

## 5. 404 responses carry no `Content-Type`

`epub_response` (`src/epub.rs:59`) builds the miss branch without one. Harmless in practice
— the body is empty — but it means a miss is served as an untyped blob, which is exactly
the shape that produced the sandboxing/download error on the success path.

```rust
None => builder
    .status(404)
    .header("Content-Type", "text/plain; charset=utf-8")
    .body(Vec::new())
    .expect("404 always valid response"),
```

While you are in there, `epub_response` has no test at all. One covers both this and the
cache header:

```rust
#[test]
fn a_served_resource_is_typed_and_never_cached() {
    let epub = Epub::open(crate::BOOK).expect("open fixture book");
    let hrefs = spine_hrefs(&epub).expect("fixture spine");

    let response = epub_response(serve_epub_resource(&epub, &format!("/{}", hrefs[2])));

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
    let epub = Epub::open(crate::BOOK).expect("open fixture book");

    let response = epub_response(serve_epub_resource(&epub, "/OEBPS/nope.xhtml"));

    assert_eq!(response.status(), 404);
    assert!(response.headers().contains_key("Content-Type"));
}
```

---

## 6. Test hygiene

Two nits, both in `src/epub.rs`:

- **`:503`** — the assert message reads *"expected the cover wrapper at spine index 0"*
  while the test indexes `hrefs.get(2)` and asserts on a chapter filename. The message is
  left over from an earlier version. It should say what it checks, e.g.
  `"expected the first story at spine index 2, got {href}"`.
- **`:554`** — `spine_hrefs_are_read_without_decompressing_chapters` does not verify the
  claim in its name; it asserts the count, a suffix, and that nothing is rooted. Either
  rename it to `spine_hrefs_are_relative_zip_paths_in_reading_order`, or make the name true
  by asserting `spine_hrefs` returns no document bodies — which it structurally cannot,
  since it returns `Vec<String>` of hrefs. Renaming is the honest option.

---

## 7. Deferred: read the media type from the manifest

Not urgent — `"xml"` in the extension map got Hamlet working. Recorded so it is not lost.

`content_type_for` guesses from the file extension. Two classes of book still break:

- **Extensions the map does not know**, including uppercase (`.XHTML` — the match is
  case-sensitive) and content documents with no extension.
- **Books declaring `text/html`** for their content documents. `serve_epub_resource` tests
  `content_type == XHTML` by exact string, so those skip the rewrite-and-inject branch and
  get served as `application/octet-stream` — the same download error Hamlet produced.

The EPUB manifest already declares the answer for every resource; that is what `media-type`
is for. Hamlet's own OPF, for instance:

```xml
<item id="main0" href="main0.xml" media-type="application/xhtml+xml"/>
```

### The change

`media_type()` returns `&'ebook str`, borrowed from the epub, so `Served.content_type` can
no longer be `&'static str`:

```rust
pub(crate) struct Served {
    pub(crate) content_type: String,
    pub(crate) body: Vec<u8>,
}
```

and `serve_epub_resource` asks the manifest first, keeping the extension map as a fallback:

```rust
pub(crate) fn serve_epub_resource(epub: &Epub, path: &str) -> Option<Served> {
    // The manifest declares the media type for every resource; the extension is
    // only a guess. Fall back to the guess, because `by_href` matches literally
    // and will miss if the path does not spell the href exactly.
    let content_type = epub
        .manifest()
        .by_href(path.trim_start_matches('/'))
        .map(|entry| entry.media_type())
        .unwrap_or_else(|| content_type_for(path));

    // EPUB 2 books sometimes declare `text/html` for content documents; they
    // still need the path rewrite and the injected assets.
    if content_type == XHTML || content_type == "text/html" {
        let rewrite =
            EpubRewriteOptions::default().rewrite_paths(PathRewrite::prefix(EPUB_URL_PREFIX));
        let xhtml = epub.read_resource_str_with(path, &rewrite).ok()?;
        let with_assets = insert_before_head_close(&xhtml, INJECTED_ASSETS);
        return Some(Served {
            content_type: XHTML_UTF8.to_string(),
            body: with_assets.into_bytes(),
        });
    }

    let content_type = content_type.to_string(); // own it before the borrow ends
    let body = epub.read_resource_bytes(path).ok()?;
    Some(Served { content_type, body })
}
```

Two things to know before writing this:

- **`by_href` is an `O(N)` linear, case-sensitive, non-percent-decoded scan** over every
  manifest entry (rbook `manifest.rs:236-246`), compared against both `href()` and
  `href_raw()`. It runs on every request, including each image and stylesheet — which is
  the reason §2's decode and the already-collapsed duplicate `content_type_for` call matter
  more once this lands.
- **`Served.content_type` becoming `String` ripples into the tests** —
  `assert_eq!(image.content_type, "image/jpeg")` still compiles (`String == &str`), but
  `served.content_type.starts_with(…)` does too, so the blast radius is small.

A regression test needs a fixture whose content documents do not have an `.xhtml`
extension. The smallest version is a hand-built EPUB with one `.xml` content document
declared `application/xhtml+xml`, written to a `tempfile` in the test — the crate already
depends on `tempfile` for the library tests.

---

## Order to do these in

1. §1 — the loader rewrite. One file, fixes the visible bug plus §3.
2. §4 — `chapter_url` signature, since §1 touches its only caller.
3. §2 — `zip_path_for` and its test.
4. §5, §6 — response test and test hygiene.
5. §7 — when a book that needs it turns up.
