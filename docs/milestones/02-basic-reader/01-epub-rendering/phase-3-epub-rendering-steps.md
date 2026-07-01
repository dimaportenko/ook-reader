# Phase 3 — EPUB Rendering — Build Log

[← Phase doc](phase-3-epub-rendering.md) · seeds **Slice 1** of
[`../../../vision-mvp-reader.md`](../../../vision-mvp-reader.md)

Per-step build log: the crux, the step plan, and for each step the runnable check → minimal
implementation → why it works. The phase doc holds the high-level checklist; this file is
the detailed trail. Newest step appended at the bottom.

> **Note on ordering (ADR-0002).** The phase doc's "Known constraints" describe the
> *eventual* faithful renderer (sandboxed `<iframe>` + custom asset protocol). Slice 1
> deliberately starts **cruder**: raw `dangerous_inner_html`, no asset protocol, accepting a
> broken cover image and the book's CSS not loading — because for a prose novel that already
> *reads*. The iframe + asset-protocol work is the deferred "faithful styling" unlock, pulled
> in when it becomes the worst real annoyance, not before.
>
> **Update (2026-06-20) — unlock pulled forward on purpose.** The learner chose to start
> using the book's own images and CSS *now* rather than wait for the "worst annoyance"
> trigger, so the deferred work is sequenced below as the **faithful-styling arc (Steps
> 4–7)**. This is ADR-0002 working as intended: the deferral is *lifted by a deliberate
> decision*, recorded here, not by silent drift. (Step 3, "Turn pages," is independent of
> this arc until Step 7 — see the dependency note in the step plan.)

## The crux (Slice 1 — "show me the book")

The hard part of "show me the book" isn't Dioxus — it's that an EPUB is a zip of XHTML files
and you need them **in reading order**, text in hand, before a single pixel renders.
`rbook`'s `epub.reader()` hands you exactly that: an iterator over the **spine's** documents
in order, each with `.content()` (the XHTML as a string).

The insight that keeps Slice 1 small: **split the work at the Rust/UI seam.** Loading the
book into an ordered `Vec<String>` of XHTML is pure Rust you can unit-test against the real
Sherlock Holmes file. Rendering one of those strings and wiring Next/Prev is Dioxus you
eyeball. Do the testable half first.

## Step plan (smallest-first, one idea each)

1. ✅ **Load the spine into `Vec<String>`** — pure Rust, `cargo test` against the real book.
   *(rbook, `Result`/`?`, `Vec`, ownership)*
2. ✅ **Render the spine documents** — render the spine via `dangerous_inner_html` in a
   scrollable view (shipped showing *all* docs, not just `docs[current]`). Eyeballed under
   `dx serve`. *(Dioxus element + `dangerous_inner_html`)*
3. ✅ **Turn pages** — a `use_signal` index; Next/Prev mutate it, clamped to `0..docs.len()`.
   Eyeball: page through all 15 items. *(signals, event handlers, clamping)*

### Faithful-styling arc — load the book's own images & CSS (Steps 4–7)

The deferred ADR-0002 unlock, now pulled forward on purpose and split smallest-first. The
shape: **serve** the zip's bytes at a URL the webview can fetch (an asset handler), then
**point** each document's relative URLs at it, then **isolate** the document in an iframe.
Testable pure-Rust seams (Steps 4, 6) interleave with webview wiring eyeballed (Steps 5, 7).

> **Plan revised (2026-06-20) after reading the actual `dioxus-desktop 0.7.9` and `rbook
> 0.7.9` sources.** Two assumptions baked into the original Step 5/6 turned out wrong, and
> the corrections make the arc *smaller*, not bigger:
>
> 1. **No custom URL scheme, no `wry://` vs `http://wry.` platform split.**
>    `use_asset_handler` does **not** register a separate scheme. dioxus serves handler
>    assets from the app's *own* origin and routes by the **first path segment** of the
>    request URI (`protocol.rs`: `request.uri().path().split('/').nth(1)` → the handler
>    `name`). So `use_asset_handler("epub", …)` catches every request to **`/epub/…`**, and
>    the URL documents use is a plain **root-relative `/epub/…`** — identical on every
>    platform. The cross-platform scheme worry is deleted.
> 2. **rbook rewrites the URLs for us.** rbook 0.7.9 ships
>    `EpubRewriteOptions::rewrite_paths(PathRewrite::prefix("/epub/"))`, which resolves each
>    document's relative `src`/`href` against its location in the zip and rewrites e.g.
>    `../images/1.png` → `/epub/opf/data/images/1.png` (and `.inject_css(…)` too). So Step 6
>    is rbook *config*, not hand-rolled string surgery. On the return trip,
>    `epub.read_resource_bytes(path)` normalizes the path itself, so the handler just strips
>    the `/epub/` prefix and hands the rest back to rbook.

4. ✅ **Read the cover image's bytes out of the zip** — pure Rust, `cargo test` against the
   real book. *(rbook `manifest().cover_image()` / `read_bytes`, media-type, magic numbers)*
5. ✅ **Register `use_asset_handler("epub", …)`** — strip the `/epub/` prefix off
   `request.uri().path()`, read the resource bytes, `responder.respond(Response …)` with a
   `Content-Type`. Eyeball via a tiny `<img src="/epub/…">` + devtools Network. *(asset
   handler, `'static` closure ownership of the `Epub` via `Rc`, `wry::http::Response`,
   content-type)*
6. ✅ **Rewrite spine docs' resource paths with rbook** — fold an `EpubRewriteOptions` with
   `PathRewrite::prefix("/epub/")` into `load_spine`. Pure Rust, `cargo test` asserts a doc's
   hrefs now start with `/epub/` and no `../` survives. *(rbook `read_str_with` /
   `reader_builder().rewrite`, why the prefix string must equal the handler name)*
7. ✅ **Render `docs[current]` in a sandboxed `<iframe srcdoc>`** — swap the all-docs
   `dangerous_inner_html` column for one isolated item. Eyeball: cover + images render, the
   book's CSS applies *inside* the frame, app styles don't leak in or out. *(iframe `srcdoc`
   + `sandbox`, root-relative URL resolution inside srcdoc)*
8. ✅ **Render the current item as XHTML, fixing the anchor-wrap bug** — get the document in
   front of the browser's **XML** parser so self-closing `<a/>` (and its whole class) parse
   correctly. The clean "served XHTML via `iframe src`" route is **blocked on macOS** by
   dioxus's navigation guard (see the Step 8 log), so the route is a **`data:application/xhtml+xml`
   URL** instead. Split: **8a** render via `data:` URL (bug fixed; styling may lag) → **8b**
   restore CSS/images by basing their URLs on the `dioxus://` scheme. *(content-type → XML parse,
   nav guard, `data:` URLs, base64, opaque-origin subresources)*
9. ✅ **Review & refactor the rendering arc** — the review-and-refactor convention, scoped here
   to the render path (Steps 4–8), not the whole phase (pagination / internal links / sample
   epub still follow). Reconcile the churn Step 8 left (the rewrite machinery is **repurposed**
   if 8b rewrites URLs, or genuinely dead if 8b uses `<base>`), make names honest for whatever
   `load_spine` now returns (content / `data:` URLs, not `paths`), and (optional) lift the EPUB
   logic out of `main.rs`. Safety net: the post-Step-8 suite stays green + clippy clean, no
   behavior change. *(dead-code vs repurpose, naming honesty, module boundaries)*
10. ✅ **Spike CSS-column page turns inside the XHTML iframe** — inject a tiny reader CSS layer
    before building the `data:application/xhtml+xml` URL, add a separate `page` signal, and use
    `translateX` to move through columns. Eyeball under `dx serve`; this is deliberately a spike
    with no page-count clamp yet. *(data-URL injection seam, CSS multicolumn, signal reset)*
11. ✅ **Intercept internal hyperlinks** — split this into: **11a** ✅ preserve each spine
    document's EPUB path and resolve `href` strings to Rust `LinkTarget`s; **11b** ✅ wire the
    iframe → Dioxus event channel (injected bridge → `postMessage` → `document::eval` listener)
    and, after 11b-i observed the absolute rewritten href, switch the resolver to **Option A**
    (strip the route prefix, match `SpineDoc.href`) and navigate by spine index. **11c** ✅ scroll
    to the `#fragment` anchor inside the destination document (deferred from 11b).
12. ✅ **Bundle a small DRM-free sample `.epub` for testing** — make the fixture intentional and
    documented instead of relying on an ad-hoc local book path.
13. ⬜ **Review & refactor the finished EPUB rendering phase** — final phase-ending cleanup after
    pagination, links, and sample-book packaging land.

> **Sequencing 8 and 9.** Land **Step 8 (8a then 8b) to green first** (feature), then run
> **Step 9 on that green baseline** (refactor). The "green before == green after, no behavior
> change" rule applies to the *refactor half only* — it can't span Step 8, which changes
> behavior and rewrites tests on purpose. (The original "delete the rewrite test" interlock is
> moot: with `data:` URLs `load_spine` returns content again, so the content-based tests are
> back in play rather than removed.)

> **What "css/image usage" actually lands on.** Images and the book's own CSS become visible
> at the **end of Step 6** — served by the handler (5) once the docs point at it (6) — even
> in today's leaky all-docs column. **Step 7 doesn't add the styling; it isolates it** so the
> book's CSS and the app's CSS stop bleeding into each other.

> **Dependency.** Step 7 renders `docs[current]`, so it needs the `current` signal from
> **Step 3 (Turn pages)**. Steps 4–6 don't — write them now, in order. **Do Step 3 before
> Step 7.**

---

## Step 1 — load the spine into a `Vec<String>`

> **Status:** done — committed in `2f40058` (1 test green: `loads_spine_in_reading_order`).

### Runnable check (`cargo test`)

This half is pure Rust, so it gets a real test against the bundled book. Add `rbook` first
(you write `Cargo.toml` — config is yours): under `[dependencies]`,

```toml
rbook = "0.7"
```

Then a test in the same file as the function you're about to write:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const BOOK: &str =
        "book/The Adventures of Sherlock Holmes by Arthur Conan Doyle.epub";

    #[test]
    fn loads_spine_in_reading_order() {
        let docs = load_spine(BOOK).expect("should open the bundled epub");

        // This book's spine is 15 documents: cover, PG header, 12 stories, PG footer.
        // If you get a different number, that's a real finding about what `reader()`
        // iterates — adjust to what's true, but it should be deterministic for this file.
        assert_eq!(docs.len(), 15);

        // Reading *order*, not manifest order: the first story's title is present,
        // and it is NOT at index 0 (index 0 is the cover).
        assert!(
            docs.iter().any(|d| d.contains("A Scandal in Bohemia")),
            "expected the first story's text somewhere in the spine"
        );
        assert!(
            !docs[0].contains("A Scandal in Bohemia"),
            "index 0 should be the cover, not story one"
        );
    }
}
```

`cargo test` fails to compile first (no `load_spine`) — that's your red. The test opens the
book by a path relative to the crate root, which is where `cargo` runs tests from.

### Minimal implementation

```rust
use rbook::Epub;

/// Open an EPUB and collect its spine documents' XHTML, in reading order.
fn load_spine(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let epub = Epub::open(path)?;

    let mut docs = Vec::new();
    for entry in epub.reader() {
        let data = entry?; // each item is a Result — propagate read errors
        docs.push(data.content().to_string());
    }
    Ok(docs)
}
```

### Why it works

- **`Epub::open(path)?`** parses the zip and returns a `Result`. `?` unwraps the `Epub` or
  returns early on error. Because the function's error type is `Box<dyn std::error::Error>`,
  `?` *coerces* rbook's error into that boxed trait object — that is how one function
  propagates several different error types without naming each one. (Tightening this to
  rbook's concrete error type is a later tidy.)
- **`epub.reader()`** yields the spine's readable documents **in reading order**, each as a
  `Result`. Iterating the *spine* (not the manifest) is what makes the order meaningful —
  cover → header → stories → footer, the exact sequence Next will walk.
- **`let data = entry?;`** — each yielded item is itself a `Result` (reading a zip entry can
  fail), so it gets its own `?`.
- **`data.content().to_string()`** — `.content()` is the document's XHTML as text;
  `.to_string()` makes an **owned** `String` so it can live in the `Vec` after the iterator
  and the `epub` are dropped. The `Vec<String>` owns all its data and outlives the function.
- **Returning `Vec<String>`** — deliberately *not* returning the `Epub` or a borrow into it.
  Clean owned data crosses the Rust/UI seam; the UI step (next) just indexes a `Vec`.

### Scope note

No rendering yet (Step 2) and no current-index state (Step 3). We load *all* document text
eagerly into memory — fine for one ~380 KB book; lazy/by-index loading is a later concern if
big books ever bite. The broken cover at index 0 is expected and shows up in Step 2/3.

---

## Step 2 — render the spine documents

> **Status:** done — committed in `2f40058` (visual: `cargo clippy` clean + `dx serve`
> render confirmed).

### Runnable check (`dx serve`)

This half is the Rust/UI seam crossed: there's no unit test, you *eyeball* it. `cargo check`
and `cargo clippy` must build clean, then under `dx serve` the window should show the book's
text — Sherlock Holmes prose flowing down the page. The cover at index 0 renders as a broken
image (no asset protocol yet — expected per ADR-0002), and the book's own CSS doesn't load.
That's the deliberately-crude Slice 1: it *reads*, even if it isn't yet styled faithfully.

### Minimal implementation

```rust
#[component]
fn SpineList() -> Element {
    const BOOK: &str = "book/The Adventures of Sherlock Holmes by Arthur Conan Doyle.epub";
    let docs = use_signal(|| load_spine(BOOK).expect("bundled epub should load"));

    rsx! {
        div {
            for doc in docs.iter() {
                div {
                    dangerous_inner_html: "{doc}",
                }
            }
        }
    }
}
```

And `App` now mounts `SpineList {}` in place of the old `Counter`.

### Why it works

- **`dangerous_inner_html: "{doc}"`** injects each spine document's XHTML straight into a
  `<div>` as raw markup. It's "dangerous" because Dioxus does no escaping — exactly what we
  want for trusted book content we're deliberately rendering as HTML. (The faithful, sandboxed
  `<iframe>` + asset-protocol renderer is the deferred unlock; this is the crude first cut.)
- **`for doc in docs.iter()`** renders *every* spine document at once into a single scrollable
  column — a deviation from the original Step-2 plan ("show `docs[current]`"). Showing the
  whole book is simpler and already reads; per-page navigation is Step 3's job.
- **`use_signal(|| load_spine(...).expect(...))`** runs `load_spine` once on mount and parks
  the result in a signal. The signal isn't mutated here (it could be a plain `let`), but it
  sets up the reactive state Step 3 will lean on. `.expect` panics if the bundled book fails
  to load — acceptable for a fixed, always-present fixture; real error UI comes later.

---

## Step 3 — turn pages

> **Status:** done — committed in `8903716` (4 tests green: added
> `paging_clamps_at_both_ends`). Done out of numeric order (the faithful-styling arc, Steps
> 4–6, was pulled forward first per the 2026-06-20 note), and **fused with Step 7**: the
> learner skipped Step 3's intermediate `dangerous_inner_html` div and went straight to the
> iframe, so the same commit lands both steps.

The crux of this step is **`use_hook` vs `use_signal`**. `docs` currently lives in `use_hook`
— a value that *persists* across renders but whose mutation paints nothing. Pagination needs
the opposite: a value whose **write schedules a re-render**. That's `use_signal`. Add a
`current` index signal, render only `docs[current]`, and let Next/Prev `.set` it. The one
trap is the ends — advancing past the last document would index `docs[15]` and panic — so the
"turn" logic must clamp. That clamp is pure, bug-prone logic, so this step is **test-first for
the clamp + eyeball for the wiring**: the same Rust/UI split that's kept every step small.

### Runnable check (`cargo test` for the clamp)

Extract the two "where does the index go" decisions into tiny pure functions and test them.
Add alongside the existing tests in `mod test`:

```rust
#[test]
fn paging_clamps_at_both_ends() {
    let len = 15; // this book's spine, per loads_spine_in_reading_order

    // Forward from the cover advances one document.
    assert_eq!(next_index(0, len), 1);
    // Forward at the last document stays put — this guards against indexing
    // docs[15] and panicking.
    assert_eq!(next_index(len - 1, len), len - 1);

    // Back from the middle steps down one.
    assert_eq!(prev_index(5), 4);
    // Back at the cover stays put — saturating_sub keeps 0 - 1 from underflowing.
    assert_eq!(prev_index(0), 0);
}
```

`cargo test` won't compile (no `next_index`/`prev_index`) — that's the red. The assertions
describe the *behavior* (advances, clamps, doesn't underflow), not "it compiles."

### Minimal implementation

Two one-line helpers, then rewrite `SpineList` (replacing the all-docs `for` loop):

```rust
/// Advance toward the last document, clamped so we never index past the end.
fn next_index(current: usize, len: usize) -> usize {
    (current + 1).min(len.saturating_sub(1))
}

/// Step back toward the cover, clamped so we never underflow past zero.
fn prev_index(current: usize) -> usize {
    current.saturating_sub(1)
}

#[component]
fn SpineList() -> Element {
    let docs = use_hook(|| load_spine(BOOK).expect("bundled epub should load"));
    let mut current = use_signal(|| 0usize);
    let len = docs.len();

    rsx! {
        div {
            button {
                onclick: move |_| current.set(prev_index(current())),
                "Prev"
            }
            button {
                onclick: move |_| current.set(next_index(current(), len)),
                "Next"
            }
            div {
                dangerous_inner_html: "{docs[current()]}",
            }
        }
    }
}
```

Under `dx serve`: the cover shows alone; **Next** walks cover → … → last story and **stops**
at the last doc; **Prev** walks back and **stops** at the cover; no panic at either end.
`cargo clippy` clean.

### Why it works

- **`use_signal(|| 0usize)` is the whole point.** `use_hook` would hold the index too, but
  writing it wouldn't repaint. A signal's `.set()` marks the component dirty, so the next
  render reads the new `current` and swaps the document. **Reading `current()` in the rsx is
  what *subscribes* this component** — that subscription is the wire from click to repaint.
- **`let len = docs.len();` before the closures is the ownership move that matters.** The
  handlers are `move` closures; a closure saying `docs.len()` directly would move the whole
  `Vec<String>` into itself, leaving the second closure and the `docs[current()]` in the rsx
  with nothing. Pulling `len` out first means the closures capture a plain `usize` (`Copy`, so
  each gets its own) and `docs` stays owned by the function body for the render to index.
  Capture the small `Copy` fact you need, not the big owned thing.
- **`current` is `Copy`, so both `move` closures can capture it.** A `Signal` is a cheap
  handle, not the data, so copying it into each handler is fine — both copies point at the
  same underlying state. That's why one `let mut current` feeds two `move` closures.
- **`saturating_sub` vs `- 1`.** `prev_index(0)` must not compute `0 - 1` — on `usize` that
  panics in debug and wraps to a huge number in release. `saturating_sub(1)` floors at 0;
  `len.saturating_sub(1)` is the last valid index and `.min()` refuses to pass it. The test
  pins both ends precisely because these are the classic off-by-one panics.

### Scope note

- **`use_hook` clones the whole `Vec<String>` every render.** Free in Step 2 (one render);
  now every page-turn re-render clones all 15 documents' XHTML. Harmless for this ~380 KB
  book, but it's exactly what the phase-ending review-and-refactor step should catch — parking
  `docs` in a signal and reading by index would drop the clone. Noted here, not fixed now.
- **This unblocks Step 7.** The sandboxed-iframe capstone renders `docs[current()]` and needs
  this `current` signal — that's the dependency flagged in the step plan above.
- If a page visibly doesn't swap on click (raw-HTML diffing can be finicky), add
  `key: "{current()}"` to the content `div` to force a fresh element. Try without first.

---

## Step 4 — read the cover image's bytes out of the zip

> **Status:** done — committed in `e25cd38` (3 tests green: added
> `reads_cover_image_bytes`).

The visible symptom this whole arc fixes is the **broken cover at index 0**, so start exactly
there: prove you can pull the cover image's *bytes* out of the EPUB zip. That's pure Rust
against the real book — a `cargo test` seam — and the bytes you read here are the exact bytes
Step 5's protocol handler will hand the webview. (This is the same Rust/UI split that kept
Slice 1 small: get the testable byte-wrangling right before touching the webview.)

### Runnable check (`cargo test`)

Add this test alongside `loads_spine_in_reading_order`. It uses **only** rbook calls confirmed
against the 0.7 docs — `manifest().cover_image()`, `read_bytes()`, `media_type()`:

```rust
#[test]
fn reads_cover_image_bytes() {
    let epub = Epub::open(BOOK).expect("should open the bundled epub");

    // The manifest is the EPUB's file table; `cover_image()` is rbook's shortcut to the
    // entry the OPF marked as the cover, so we don't have to guess its href.
    let cover = epub
        .manifest()
        .cover_image()
        .expect("this book declares a cover image");

    let bytes = cover.read_bytes().expect("should read the cover bytes out of the zip");

    // Assert on the *bytes*, not just that it's Ok: a real image starts with a known magic
    // number. JPEG → FF D8 FF; PNG → 89 50 4E 47. If neither, you didn't get image data.
    let is_jpeg = bytes.starts_with(&[0xFF, 0xD8, 0xFF]);
    let is_png = bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]);
    assert!(is_jpeg || is_png, "cover should be a JPEG or PNG, got {} bytes", bytes.len());

    // The media type is what Step 5 will echo into the `content-type` header so the webview
    // decodes the image correctly — confirm it's available and is an image type.
    assert!(
        cover.media_type().starts_with("image/"),
        "cover media-type should be an image/* type"
    );
}
```

`cargo test` won't compile until the rbook calls resolve — that's your red. If a method name
differs in your exact rbook 0.7.x (the manifest/resource API moved around pre-1.0), check
[docs.rs/rbook](https://docs.rs/rbook/latest/rbook/) and adjust to what's actually there;
the *shape* (manifest → entry → bytes + media-type) is the part that's stable.

### Minimal implementation

There's barely any of *your* code here — rbook does the zip reading. The point of the step is
to confirm the API shape works end-to-end on the real file. If you want a named seam for Step
5 to call, wrap it:

```rust
/// Read one of the EPUB's own resources by manifest href, returning the raw bytes plus the
/// media type Step 5 needs for the `content-type` header.
fn read_resource(epub: &Epub, href: &str) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    // rbook's `Ebook` trait reads a resource's bytes straight from a path/href; confirm the
    // exact name (`read_resource_bytes`) and how to get the media-type by href in the 0.7
    // docs. For the cover specifically you already have `manifest().cover_image()`.
    let bytes = epub.read_resource_bytes(href)?;
    let media_type = /* look the manifest entry up by href and call media_type() */ todo!();
    Ok((bytes, media_type))
}
```

Leaving the media-type lookup as a `todo!()` is honest: the test above proves the *cover*
path works with confirmed calls; generalizing to an arbitrary href is Step 5's concern, and
you'll pin the exact lookup method then rather than guess now.

### Why it works

- **`manifest().cover_image()`** returns an `Option` because not every EPUB declares a cover
  (this one does). Going through the manifest means you read the file the book *says* is the
  cover, instead of hard-coding a path that varies book to book.
- **`read_bytes()?`** pulls the entry's bytes *out of the zip* into an owned `Vec<u8>`. It's
  fallible (a malformed or missing entry yields an `EbookError::Archive`), so it takes a `?`.
  Those bytes are exactly what the webview will receive in Step 5.
- **The magic-number assertion** is what makes this a real check rather than "it didn't
  error": it proves the bytes are genuine decoded image data from the zip, not an empty or
  placeholder buffer.
- **`media_type()`** is grabbed here because the webview won't render an image it's served
  without the right `content-type`; Step 5 copies this string into the response header.

### Scope note

Step 4 only **reads** bytes — nothing renders, no webview involved. Serving them over a URL
the webview can fetch is **Step 5** (`use_asset_handler`); pointing the document's relative
`<img>`/`<link>` URLs at that protocol is **Steps 6–7** (rewrite + iframe). The broken cover
stays broken on screen until Step 7 — Step 4 just proves the bytes are reachable.

When this is green, the rest of the arc — Steps 5, 6, 7 — is laid out below.

---

## Step 5 — register `use_asset_handler("epub", …)`

> **Status:** done — committed in `e25cd38` (visual: `cargo clippy` clean +
> `dx serve` cover/asset render confirmed). The `'static` handler closure owns
> an `Rc<Epub>` (`Epub` isn't `Clone`, `Rc<Epub>` is — which `use_hook`
> requires); the temporary `<img>` probe was removed once verified.

Step 4 proved you can pull a resource's bytes out of the zip. Step 5 puts those bytes on a
URL the webview can fetch. The key fact (confirmed from the `dioxus-desktop 0.7.9` source,
not assumed): an asset handler is **not** a separate URL scheme. dioxus routes asset requests
by the **first path segment** of the request URI to the handler with that `name`, all on the
app's own origin. So a handler named `"epub"` answers every request whose path starts with
`/epub/`, and the document just uses a root-relative `/epub/…` URL — the same string on macOS,
Linux, Windows. No `wry://` vs `http://wry.` branching.

### Runnable check (`dx serve` + devtools)

This is webview wiring, so it's an eyeball check, not a unit test. Drop **one** literal image
into the rsx, pointing at the cover via the handler, and confirm it renders:

```rust
// somewhere visible in the rsx, just for this step:
img { src: "/epub/{cover_href}" }   // cover_href from Step 4's manifest().cover_image().href()
```

Pass criteria under `dx serve`:

- The cover image **renders** (not a broken-image icon).
- In devtools → Network, the request to `/epub/…` returns **200** with a `Content-Type:
  image/*`.
- `cargo clippy` is clean.

If you'd rather de-risk routing before resource lookup: have the handler ignore the path and
always respond with the Step-4 cover bytes, and `eprintln!("{}", request.uri().path())` so you
*see* `/epub/…` arriving. Once the image shows, switch to real path→resource mapping. Either
order is fine; the green light is "a real image from the zip appears in the window."

### Minimal implementation

```rust
use dioxus::desktop::{use_asset_handler, wry::http::Response};

// inside a component (App is fine — it mounts once):
use_asset_handler("epub", move |request, responder| {
    // dioxus already matched the "epub" segment; strip it to get the in-zip path.
    // "/epub/opf/data/images/cover.jpg" -> "/opf/data/images/cover.jpg"
    let path = request.uri().path().strip_prefix("/epub").unwrap_or_default();

    match epub.read_resource_bytes(path) {
        Ok(bytes) => {
            let body = Response::builder()
                .header("Content-Type", content_type_for(path)) // see note below
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
```

### Why it works

- **`use_asset_handler(name, handler)`** registers `handler` under `name`. When a webview
  request's path is `/epub/…`, dioxus's protocol layer splits the path, sees `epub`, finds
  your handler, and calls it. That's the entire "custom protocol" — no scheme registration.
- **The closure is `'static` (`FnMut + 'static`).** It outlives the render, so it cannot
  *borrow* a local `Epub` — it must **own** one. That's the real design decision this step
  forces: open an `Epub` and `move` it into the closure (or wrap it so it can be shared with
  `load_spine`). Re-opening the zip on every request also compiles, but it's wasteful; owning
  one `Epub` for the handler's lifetime is the better default. Whichever you pick, name *why*
  in a comment — `'static` ownership is the lesson here.
- **`strip_prefix("/epub")`** leaves a root-relative `/opf/…` path. `read_resource_bytes`
  *normalizes and resolves* that itself (confirmed in `epub.rs::transform_resource`), so you
  don't have to reconcile it against manifest hrefs — feed it the stripped path and it finds
  the entry.
- **`Content-Type`** matters: the webview won't decode an image (or apply a stylesheet) served
  without the right type. Two honest options — look the entry up
  (`manifest().by_href(path).media_type()`, but note `by_href` is an exact, *un-normalized*
  match so the stripped path may not hit it) or derive the type from the file extension
  (`.jpg`/`.png`/`.css` → the obvious MIME). Pin whichever actually works against this book;
  the extension map is the more robust default.

### Scope note

Step 5 serves *any* `/epub/…` request, but nothing in the book points at `/epub/…` yet — the
spine docs still carry their original `../images/…` URLs. So the broken cover *inside the
book* stays broken; you only see the image via the hand-written `<img>` test tag. Making the
documents themselves request `/epub/…` is **Step 6**. Remove the test `<img>` once Step 6
lands.

---

## Step 6 — rewrite spine docs' resource paths with rbook

> **Status:** done — committed in `e25cd38` (3 tests green: added
> `rewrites_resource_paths_to_the_epub_handler`). `load_spine` now reads each
> doc via `manifest_entry().read_str_with(&rewrite)` with
> `PathRewrite::prefix("/epub/")`; the all-docs column now shows the book's
> images and CSS (isolation is Step 7).

Now point the documents at the handler. This was planned as hand-rolled string rewriting, but
rbook 0.7.9 does it natively: `EpubRewriteOptions::rewrite_paths(PathRewrite::prefix("/epub/"))`
resolves each relative URL against the document's spot in the zip and rewrites it to
`/epub/<full-zip-path>` — exactly the shape Step 5's handler answers. Back to a pure-Rust seam,
so it gets a real `cargo test`.

### Runnable check (`cargo test`)

Extend `load_spine` to apply the rewrite, then assert the URLs actually changed. The cover
document (spine index 0) references the cover image, so it's a good probe:

```rust
#[test]
fn rewrites_resource_paths_to_the_epub_handler() {
    let docs = load_spine(BOOK).expect("should open the bundled epub");

    // At least one document now points image/CSS URLs at the asset handler …
    assert!(
        docs.iter().any(|d| d.contains("/epub/")),
        "expected rewritten resource URLs under /epub/"
    );

    // … and the unresolved relative form is gone from that document.
    let cover_doc = &docs[0];
    assert!(
        cover_doc.contains("/epub/"),
        "the cover document should reference the cover image via /epub/"
    );
    assert!(
        !cover_doc.contains("../"),
        "no unresolved OPF-relative paths should remain in the cover document"
    );
}
```

Red first: it fails until `load_spine` applies the rewrite. (If `docs[0]` turns out not to be
the doc that carries an image, adjust the probe to whichever document does — the existing
`loads_spine_in_reading_order` test already tells you what's at each index.)

### Minimal implementation

Two ways; pick one. **Per-document** (smallest change to today's loop):

```rust
use rbook::epub::rewrite::{EpubRewriteOptions, PathRewrite};

fn load_spine(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let epub = Epub::open(path)?;
    let rewrite = EpubRewriteOptions::default().rewrite_paths(PathRewrite::prefix("/epub/"));

    let mut docs = Vec::new();
    for entry in epub.reader() {
        let data = entry?;
        docs.push(data.manifest_entry().read_str_with(&rewrite)?); // was: data.content().to_string()
    }
    Ok(docs)
}
```

**Reader-wide** (rbook applies the rewrite to every spine doc itself):

```rust
let rewrite = EpubRewriteOptions::default().rewrite_paths(PathRewrite::prefix("/epub/"));
let reader = epub.reader_builder().rewrite(rewrite).create();
// then iterate `reader` and take each doc's content as before
```

Confirm the exact accessor names (`manifest_entry()`, `read_str_with`, `reader_builder`)
against your rbook — they're in the 0.7.9 source, but pin the call that compiles.

### Why it works

- **`PathRewrite::prefix("/epub/")`** is the seam between the two halves: rbook turns
  `../images/1.png` into `/epub/opf/data/images/1.png`, and Step 5's handler is registered as
  `"epub"`, so that URL routes straight to it. **The prefix string and the handler name are
  the same contract** — change one, change the other, or the requests 404.
- **rbook resolves relative to the document's location**, not the package root, so a link from
  a deeply-nested chapter resolves correctly without you tracking directories. That correct
  resolution is precisely the part hand-rolled string replacement gets wrong.
- **`read_str_with` returns `EbookResult`** (rewriting can fail on malformed markup), so it
  takes a `?` — note it's a *different* error type than `read_bytes`' `ArchiveResult`, which is
  why `load_spine`'s boxed error return earns its keep again.

### Scope note

After Step 6 the all-docs `dangerous_inner_html` column **shows images and applies the book's
CSS** — that's the "css/image usage" goal, reached. But because everything still renders into
one shared page, the book's CSS is global: it styles the whole app and the app's styles bleed
into the book. Fixing that isolation is Step 7 — and Step 7 needs the `current` index from
**Step 3**, so do Step 3 next if you haven't.

---

## Step 7 — render `docs[current]` in a sandboxed `<iframe srcdoc>`

> **Status:** done — committed in `8903716` together with Step 3 (4 tests green; visual:
> `cargo clippy` clean + `dx serve` confirmed — pages turn, the book's images/CSS render
> *inside* the frame, app styles don't leak in or out). The fully-restricted `sandbox=""`
> hit exactly the opaque-origin edge this step's scope note warns about (subresources didn't
> load), so the sandbox was relaxed to **`allow-same-origin`** — keeping book scripts inert
> while letting the `/epub/…` subresources resolve.

The capstone: stop dumping every document into the app's own DOM and render the *current* one
inside an isolated `<iframe>`. The iframe is a separate document, so the book's CSS is scoped
to it (no leak out) and the app's CSS doesn't reach in (no leak in) — the "style isolation"
the phase doc's Known Constraints call for.

### Runnable check (`dx serve`)

Eyeball, with three things to verify at once:

- The current chapter renders, and its **images + the book's CSS apply inside the frame**
  (carried over from Step 6, now scoped).
- An obvious **app-level style** (e.g. a lurid `body { background }` in the app) does **not**
  bleed into the chapter, and the book's CSS does **not** restyle the app chrome around it.
- Next/Prev (Step 3) swaps the iframe to the next document.

`cargo clippy` clean.

### Minimal implementation

```rust
// replacing the `for doc in docs.iter()` column:
iframe {
    sandbox: "",                       // no allow-scripts: book JS stays inert
    srcdoc: "{docs.read()[current()]}", // the current document's (rewritten) XHTML
}
```

### Why it works

- **`srcdoc`** hands the iframe a full HTML string to render as its own document — that's the
  rewritten XHTML from Step 6, `/epub/…` URLs and all.
- **`sandbox: ""`** (empty, so *no* tokens — notably no `allow-scripts`) renders content but
  blocks scripts and other escalations. Book content is trusted-ish, but there's no reason to
  let it run JS, so deny it.
- **Root-relative `/epub/…` URLs still resolve to the handler.** A `srcdoc` document inherits
  its base URL from the embedding page, so `/epub/…` resolves against the app origin and hits
  your handler — the same path that worked in the leaky column keeps working once isolated.
- **`current()`** is the Step 3 signal; reading it here subscribes the component, so Next/Prev
  re-render the iframe with the new document. That subscription is why Step 7 *needs* Step 3.

### Scope note — watch for the sandbox edge

A sandboxed iframe can get an **opaque origin**, and depending on the webview that can affect
whether subresource (`/epub/…`) requests fire as you expect. If images/CSS load in the leaky
column (Step 6) but go missing once inside the sandbox, that's the thing to debug — the usual
fixes are injecting a `<base href="/">` into the srcdoc or relaxing the sandbox with
`allow-same-origin`. Treat it as an eyeball-and-adjust detail of this step, not a
re-architecture. Pagination (CSS multi-column / scroll) and internal-link navigation remain
later steps in the phase doc's checklist.

---

## Step 8 — render the current item as served XHTML (fixes the anchor-wrap bug)

> **Status:** done — committed in `77adf23` (4 tests green). Landed as **8a + 8b** via the
> `data:application/xhtml+xml` route (the served-XHTML route was blocked on macOS — see below).
> **8b verified styled:** under `dx serve` the chapter renders as prose (no red link) *and* the
> book's CSS + cover/images load inside the frame — confirming the open question that a
> `data:`-origin document *can* load `dioxus://…/epub/…` subresources via the asset handler.
> The theming-injection seam this leaves behind (the bytes we encode) feeds Phase 4; the
> relocation from "served response" to "encoded bytes" should be written back into
> [ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md).

### The bug

Dogfooding Step 7 shows each chapter rendering as one giant link that turns **red on hover**.
Cause, confirmed against the bundled book: chapter `h-1` contains exactly one anchor —
`<a id="chap01"/>`, self-closing, no `href` — and **zero** `</a>`. The file is XHTML.
`iframe { srcdoc }` is parsed as **HTML (`text/html`)**, where `<a>` is *not* a void element,
so the `/>` is ignored, the anchor is treated as an unclosed `<a id="chap01">`, and with no
closing tag the rest of the chapter becomes its descendant — inheriting `1.css`'s
`a:link { color: blue }` / `a:hover { color: red }`.

This is **not** an `rbook` bug: its rewriter round-trips `<a/>` faithfully (quick-xml
`Event::Empty` in → `<a/>` out). The breakage is at *our* rendering seam — **XHTML fed to an
HTML parser**. `srcdoc` is always parsed as `text/html` and has no content type to override.
The real fix is to get the document in front of the browser's **XML** parser. There are two
ways to do that, and the first one we tried hit a wall worth recording in full — it's the
load-bearing finding of this step and it reshapes Phase 4.

### Attempt A — served XHTML via `iframe src` (blocked by dioxus's nav guard)

The plan: teach the handler to serve content docs as `application/xhtml+xml`, have `load_spine`
return spine **paths**, and point the iframe at `src="/epub/{path}"`. We built it
(`content_type_for` gained an `xhtml` arm; `load_spine` returned `href()`; iframe used `src`).
It rendered **nothing** — an empty `<html><head></head><body></body></html>` — and an
`eprintln!` at the top of the asset handler showed it was **never called** for the iframe's
document request. (Subresource requests *do* reach it — hold that asymmetry.)

Two real bugs were found and fixed along the way (worth keeping, they're correct):

- **Double slash.** `href()` returns a leading-slash *absolute* zip path (`/OEBPS/wrap0000.xhtml`),
  so `"/epub/{path}"` produced `/epub//OEBPS/…`. After `strip_prefix("/epub")` that left
  `//OEBPS/…`, which rbook's `transform_resource` couldn't resolve. Fix: `"/epub{path}"` (no
  slash — the path already has one). Confirmed with a probe test:
  `read_resource_bytes("/OEBPS/…")` → `Ok(54958 bytes)`.
- **Content type irrelevant.** Even served as `text/html`, still blank — so the MIME wasn't the
  blocker.

The actual root cause is in `dioxus-desktop 0.7.9` (`webview.rs:369`):

```rust
.with_navigation_handler(move |var| {
    if var.starts_with("dioxus://") || var.starts_with("http://dioxus.") || ... {
        // After the page has loaded once, don't allow any more navigation
        let page_loaded = page_loaded.swap(true, Ordering::SeqCst);
        return !page_loaded;          // first load: true; every later nav: FALSE
    }
    ... // data:/blob:/external fall through to the allow branch
})
```

On macOS the app runs under the custom scheme `dioxus://index.html/` (`webview.rs:368`,
`protocol.rs:22`), so `/epub/…` resolves to `dioxus://index.html/epub/…`. The **first**
navigation (the app itself) is allowed and flips `page_loaded` to `true`; **every** later
`dioxus://` navigation — including an iframe's `src` — returns `false` and is **blocked**. The
frame loads nothing and the handler never runs. *Subresources* (img/css) aren't "navigations,"
so they bypass this guard and reach the protocol handler — which is exactly why Step 7's images
worked but a top-level iframe navigation does not.

Two dead ends fall out, recorded so we don't retry them:

- **Can't override the guard from user code.** The `dioxus://` branch returns *before* the
  user-supplied `navigation_handler` is ever consulted.
- **Can't switch to an `http(s)` origin on macOS.** The URL is hardcoded to `dioxus://`, and
  WKWebView forbids registering a scheme handler for real `http`/`https` (Apple's restriction —
  the very reason dioxus minted `dioxus://`). "Serve over http" is a Windows/Linux option, not a
  macOS one. (This is the original "Option 2"; it's a dead end here.)

### The fix — `data:application/xhtml+xml` URL

The nav guard only blocks `dioxus://` (and `http(s)://dioxus.`) URLs. A **`data:` URL** falls
through to the allow branch, so the iframe *can* navigate to it — and a
`data:application/xhtml+xml` document is handed to the browser's **real XML parser**. A one-line
spike confirmed both at once:

```rust
src: "data:application/xhtml+xml,<html xmlns=\"http://www.w3.org/1999/xhtml\"><body><p>hello <a id=\"x\"/>world</p></body></html>",
```

→ rendered `hello world` as plain text, and the inspector showed `<a id="x"></a>` — the parser
**self-closed the anchor** (the text is a sibling, not swallowed). That fixes the anchor-wrap
bug at the parser level, and — unlike a hand-rolled `<a/>`→`<a></a>` transform — it fixes the
*whole class* of self-closing-non-void elements for free, because a real XML parser is doing the
parsing. That "general, not whack-a-mole" property is the reason we chose it.

The cost `data:` imposes: an opaque origin, so the document's relative resource URLs (`0.css`,
images) have no base to resolve against. That splits the work cleanly:

- **Step 8a — render the current item as a `data:` XHTML URL.** Bug fixed; CSS/images may be
  missing for now. Prose renders, no red link, paging works.
- **Step 8b — restore subresources** by giving those relative URLs an absolute base on the
  `dioxus://` scheme (which the handler *does* serve for subresources).

### Step 8a — runnable check

Eyeball under `dx serve`:

- The chapter renders as **normal prose**, not a blue link; **hover does not turn it red**.
- Next/Prev still page; `cargo clippy` clean.
- (CSS/images may be unstyled/broken — that's 8b. The *bug* is what 8a proves fixed.)

Pure-Rust half — `load_spine` goes **back to returning document content** (we need the bytes to
build the `data:` URL; the Step-8 paths version is superseded). Test the content, or the built
URL if you build it in `load_spine`:

```rust
#[test]
fn builds_xhtml_data_urls_in_reading_order() {
    let docs = load_spine(BOOK).expect("should open the bundled epub");
    assert_eq!(docs.len(), 15);
    assert!(docs[0].starts_with("data:application/xhtml+xml")); // each item is a ready data: URL
}
```

(If you keep `load_spine` returning raw content and build the URL in a helper, test the helper
instead — assert it round-trips, e.g. base64-decoding the payload gives back the XHTML.)

### Step 8a — minimal implementation (sketch; you write it)

1. **`load_spine` returns content again.** Revert it to collect `data.content()` (or
   `read_str_with` if you keep a rewrite for 8b), not `href()`.
2. **Build the `data:` URL.** A helper turning XHTML into a `data:` URL. base64 is the robust
   encoder (raw markup makes percent-encoding fiddly) — add the `base64` crate:

   ```rust
   fn to_xhtml_data_url(xhtml: &str) -> String {
       use base64::{engine::general_purpose::STANDARD, Engine};
       format!("data:application/xhtml+xml;base64,{}", STANDARD.encode(xhtml))
   }
   ```

3. **Point the iframe at it.** `src: "{docs[current()]}"` (if `load_spine` returns finished
   URLs) or `src: "{to_xhtml_data_url(&docs[current()])}"`.

### Step 8b — restore subresources

The relative URLs need an absolute base on the `dioxus://` scheme. Two candidate mechanisms —
and this is the part **still to verify**, because *whether a `data:`-origin document may load
subresources from a custom scheme* is unconfirmed (the spike had none):

- **Inject `<base href="dioxus://index.html/epub/OEBPS/">`** into the head before encoding, so
  `0.css` → `dioxus://index.html/epub/OEBPS/0.css` — a subresource request the handler answers.
- **Or rewrite the URLs to absolute** with `PathRewrite::prefix("dioxus://index.html/epub")`
  (the machinery Step 9 was going to delete — **repurposed**, not dead, with an absolute prefix).

Runnable check (eyeball): the book's CSS applies and the cover/images show **inside** the frame;
devtools → Network shows `dioxus://index.html/epub/OEBPS/0.css` returning 200.

**Fallback if subresources are blocked from the `data:` origin** (the live risk): make the
document self-contained — inline the CSS as a `<style>` block and images as `data:` URIs, read
through the existing handler path. Heavier, but origin-proof. Decide once 8b's eyeball says which
world we're in.

### Why it works

- **The MIME type picks the parser.** `data:application/xhtml+xml` → XML parser → every
  self-closing non-void element (`<a/>`, `<span/>`, …) is honoured, not just the one we noticed.
  A `srcdoc`/`text/html` path can never do this; it always parses as HTML.
- **`data:` dodges the navigation guard** that blocks `dioxus://` iframe navigations — the only
  reason the cleaner `src="/epub/…"` approach can't work on macOS.
- **Phase 4 still has its seam, in a new spot.** ADR-0003 assumed theme CSS is injected into the
  *served HTTP response*; that seam is dead here (there is no served document). But we build the
  `data:` bytes in Rust, so theme CSS is injected **before encoding** — same power, different
  place: the injection point moves into `load_spine` / the URL builder.

### Scope & ADR-0003 note

This replaces the Step-8 plan-of-record (served XHTML via `src`) with `data:` XHTML, because the
dioxus navigation guard blocks custom-scheme iframe navigation on macOS. That finding should be
written back into
[ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md): the theming injection
point is now "the bytes we encode into the `data:` URL," not "the asset-handler response."
Pagination and internal-link navigation remain later Phase-3 steps — and **internal links will
need rethinking**, since a `data:`-framed document can't navigate to `dioxus://` either (same
guard), so in-book links will have to be intercepted and turned into spine-index changes rather
than left as plain navigations.

---

## Step 9 — review & refactor the rendering arc

> **Status:** done — committed in `70b91df` (4 tests green, clippy clean; same 4 green before
> and after, so the move was behaviour-preserving). Landed punch-list items **1** (one
> `EPUB_ROUTE` const, replacing the three `/epub` literals), **3 + 5** (lifted `load_spine`,
> `content_type_for`, `to_xhtml_data_url` and their tests into `src/epub.rs` as `pub(crate)`),
> plus the `collect::<Result<Vec<_>, _>>()` rewrite of `load_spine`. **Deferred** (optional
> polish, no churn risk): `SpineList` → an honest name, the two `.body(…).unwrap()` →
> `.expect(…)`, and `#![allow(non_snake_case)]` → `#[expect(…, reason = …)]`. The
> review-and-refactor convention applied to the **render path (Steps 4–8)**, run right after
> Step 8. **Scoped to the rendering arc, not the whole phase** — pagination, internal-link
> navigation, and bundling a sample epub still follow, and the phase gets its *true* final
> review when they land.

### The discipline (why the ordering matters)

A review-and-refactor's safety net is "the suite is green and clippy clean **before and
after**, with no behavior change." That rule **cannot span Step 8** — Step 8 changes behavior
and the `load_spine` shape on purpose. So sequence it: **land Step 8 (8a+8b) to green first**
(feature), then refactor on that green baseline (renames, module moves, dead-code removal change
*no* assertion).

### Runnable check (safety net, not a target)

After Step 9: `cargo test` green and `cargo clippy` clean, with **no** visible change under
`dx serve` (chapters render as prose, Next/Prev page, the book's CSS + images show inside the
frame). The exact test set depends on what 8a/8b landed — run it before and after the refactor
and confirm it's identical.

### Punch-list (adapt to what 8a/8b actually left)

The Step 8 detour churned several things; this is the reconciliation:

1. **Rewrite machinery — repurposed (8b chose the rewrite path).** `load_spine` keeps
   `EpubRewriteOptions` / `PathRewrite::prefix("dioxus://index.html/epub/")` to point subresource
   URLs at the asset handler, so the import is **alive** — the `<base href>` alternative wasn't
   used. Cleanup here is to stop the strings drifting: the rewrite prefix, the asset-handler name
   (`"epub"`), and the `strip_prefix("/epub")` in the handler all encode the same `/epub` contract
   three times — lift it to one shared constant (or a pair) so changing the route can't silently
   404. Confirm the `dioxus://index.html/` host literal is the right seam to centralise too.
2. **Name things for what `load_spine` now returns.** With `data:` URLs it returns content or
   finished `data:` URLs, **not** `paths` — so the `SpineList` binding and any `current_*` local
   should say `doc` / `data_url`, not `path`. *Why:* a name that lies about its type costs every
   future reader; this churned twice (content → paths → content), so pin it honestly now.
3. **The `data:` URL helper.** If `to_xhtml_data_url` (or wherever encoding lives) grew inline in
   `SpineList`, lift it to a named function next to `load_spine` — encoding is EPUB-domain logic,
   not UI. Confirm the `base64` dep is actually used (no stray `EpubRewriteOptions` left over,
   no unused `base64` if you went `<base>`).
4. **Stale tests / probes.** Delete any `zz_probe_*` diagnostic tests left from debugging, and
   make sure the surviving spine test matches the real return type (content vs `data:` URL).

Optional — bigger, cut it if you want the pass light:

5. **Lift the EPUB logic into `src/epub.rs`** — `load_spine`, `content_type_for`, the
   `data:`-URL builder, the resource-reading concern — leaving `App`/`SpineList` (the UI) in
   `main.rs`. *Buys:* a real Rust/UI module boundary instead of one catch-all file. *Watch:* the
   `#[cfg(test)] mod test` block calls `load_spine`/`Epub` directly, so moving the functions
   means the tests move with them or `use crate::epub::load_spine` — that `pub`/`use` shuffle
   *is* the lesson here. Defer it if the pass is getting long.
6. **Minor:** the commented-out `MAIN_CSS` / `document::Link` is dead scaffolding — wire it or
   delete it.

### Why each is better

- **A name that lies about its type costs every future reader** — `path` on a value that now
  holds a whole `data:` document is a trap; this one churned twice, so it's worth pinning.
- **Domain logic out of the component** — encoding and EPUB reading don't belong in `SpineList`;
  pulling them out makes the component about *rendering state*, which is all a reader should need
  to follow to change the UI.
- **Dead imports/probes aren't free** — they fail clippy and tell the next reader "this matters"
  when it doesn't. The rewrite import is the one to *decide* on (repurposed vs dead), not delete
  reflexively.

### Commit shape

Either one `feat:` folding in the forced cleanup, or two commits (`feat:` then `refactor:`) —
splitting reads cleaner in history since feature and cleanup are different intents. `lbb:commit`
handles whichever. (Per repo convention: no co-author / AI-attribution trailer.)

---

## Step 10 — spike CSS-column page turns inside the XHTML iframe

> **Status:** done — committed in `9989adb` (5 tests green; `cargo clippy` clean; `dx serve` visual confirmed).

### The crux

Right now **Next/Prev are really spine-item turns**: they swap cover/header/chapter/footer
XHTML documents. True reader page turns are different: one long XHTML document must be laid out
into viewport-sized columns, then the reader moves sideways through those columns. The useful
small slice is a **spike**: inject just enough CSS into the XHTML before it becomes a `data:` URL,
track a `page` signal in Dioxus, and translate the column layout left/right. Do not solve page
count/clamping yet — first prove the rendering technique works in this `data:` iframe setup.

### Runnable check

This step has two checks.

#### 1. Pure Rust helper check (`cargo test`)

Add a tiny helper that inserts a reader-controlled `<style>` block before `</head>`, then test
that it preserves the document and includes the current page offset. Suggested test shape:

```rust
#[test]
fn injects_pagination_css_before_head_close() {
    let xhtml = r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body><p>Hello</p></body></html>"#;

    let paged = inject_pagination_css(xhtml, 2);

    assert!(paged.contains("--ook-page: 2"));
    assert!(paged.contains("column-width: 100vw"));
    assert!(paged.find("--ook-page: 2").unwrap() < paged.find("</head>").unwrap());
    assert!(paged.contains("<p>Hello</p>"));
}
```

Red first: no `inject_pagination_css` helper yet.

#### 2. UI spike check (`dx serve` + `cargo clippy`)

Under `dx serve`:

- Open a real chapter, not the cover.
- The iframe shows **one viewport-sized slice** of the chapter instead of a long vertical scroll.
- A **Page +** control shifts horizontally to the next column; **Page -** shifts back and clamps at
  page 0.
- Changing spine item with the existing chapter controls resets the page index to 0.
- `cargo clippy` is clean.

Expected rough edge: Page + can eventually move beyond the chapter and show blank space. That is
allowed in this spike because page-count measurement is the next problem, not this one.

### Minimal implementation sketch

Keep the changes small and local:

1. In `src/epub.rs`, add a helper that injects CSS into the XHTML string before encoding it:

   ```rust
   pub(crate) fn inject_pagination_css(xhtml: &str, page: usize) -> String {
       let css = format!(
           r#"<style type="text/css">
           :root {{ --ook-page: {page}; }}
           html {{ width: 100vw; height: 100vh; overflow: hidden; }}
           body {{
               width: 100vw;
               height: 100vh;
               overflow: visible;
               column-width: 100vw;
               column-gap: 0;
               column-fill: auto;
               transform: translateX(calc(var(--ook-page) * -100vw));
           }}
           </style>"#,
       );

       xhtml.replacen("</head>", &format!("{css}</head>"), 1)
   }
   ```

   This is intentionally string-based for the spike. If `</head>` is ever missing or casing
   varies, you can harden it later.

2. In `Reader`, add a separate `page` signal (`usize`) next to `current`.

3. Build the iframe URL from the paged XHTML, not the raw current doc:

   ```rust
   let paged_doc = epub::inject_pagination_css(current_doc, page());
   let iframe_src = epub::to_xhtml_data_url(&paged_doc);
   ```

   Then use `src: "{iframe_src}"`.

4. Add Page - / Page + buttons. Page - can use `saturating_sub(1)`; Page + can increment without
   an upper clamp for now.

5. When chapter Next/Prev changes `current`, also set `page` back to `0`. That keeps every new
   spine item starting at its first visual page.

### Why it works

- The **data-URL seam** is now the renderer's injection seam. Anything you need the XHTML parser
  to see — pagination CSS now, theme CSS in Phase 4 — must be inserted **before**
  `to_xhtml_data_url` encodes the bytes.
- CSS multicolumn with a fixed-height body makes overflow flow into horizontal columns instead of
  vertical scroll. `translateX(-N * 100vw)` then picks which viewport-width column is visible.
- `page` is separate from `current` because they represent different state: `current` chooses the
  spine document; `page` chooses a visual slice inside that document. Resetting `page` on chapter
  changes prevents carrying “page 7” from one chapter into the next.
- The injected style is last in the document head, so for this spike it should beat most author CSS.
  Later theming work will make the layering intentional instead of ad hoc.

### Scope note

This does **not** compute the number of pages, clamp Page + at the real end, preserve reading
position, or handle internal links. It only proves that the current `data:application/xhtml+xml`
iframe can be paginated by a reader-controlled injected CSS layer and driven by Dioxus signals.

---

## Step 11a — resolve internal link targets in Rust

> **Status:** done — committed in `bf70e44` (7 tests green; `cargo clippy
> --all-targets` clean). `load_spine` now returns `Vec<SpineDoc>` (href + xhtml);
> `resolve_internal_link` + `resolve_relative` translate an EPUB `href` to a
> `LinkTarget` (spine index + fragment), `#[allow(dead_code)]` until Step 11b
> wires it into the UI. Lint lesson recorded below: this code is used only by
> tests, so `#[expect(dead_code)]` fails under `cargo clippy --all-targets` (the
> test unit fulfils it, the bin unit doesn't) — `#[allow(dead_code)]` is the
> right tool, and annotating the entry point silences its transitively-dead
> callees too.
>
> **Superseded input model (see 11b-ii).** This step assumed clicks arrive as *relative* hrefs and
> built `resolve_relative` + a `base_dir` walk to handle them. 11b-i later observed that rbook
> rewrites `<a href>` to **absolute** `dioxus://index.html/epub/…` URLs, so that relative path never
> runs in the real app. 11b-ii revises `resolve_internal_link` to **Option A** (strip the route
> prefix, match `SpineDoc.href`) and deletes `resolve_relative`. The `SpineDoc`/`LinkTarget` types
> and the fragment-splitting logic from this step stand; only the path-resolution half changes.

### The crux

The iframe is now a `data:application/xhtml+xml` document, so letting the browser follow a book
link is the wrong primitive: cross-document links resolve toward `dioxus://…`, and those iframe
navigations are blocked by Dioxus's desktop navigation guard. Before wiring any click channel,
Rust needs a deterministic translation from an EPUB `href` like
`5186027266282590649_1661-h-1.htm.xhtml#chap01` into "spine index 2, fragment `chap01`."

That means `Vec<String>` is no longer enough. The XHTML content is not the whole spine item; each
item also needs its EPUB path, because relative links are resolved against the current document's
location.

### Runnable check (`cargo test`)

Add this beside the existing `epub.rs` tests. It should fail first because `SpineDoc`,
`LinkTarget`, and `resolve_internal_link` do not exist yet (and because `load_spine` still returns
plain strings today).

```rust
#[test]
fn resolves_contents_link_to_spine_doc_and_fragment() {
    let docs = load_spine(crate::BOOK).expect("should open the bundled epub");

    // In this Gutenberg book: spine 0 is the cover wrapper, spine 1 is the title/contents
    // document, and spine 2 is "A Scandal in Bohemia".
    let target = resolve_internal_link(
        &docs,
        1,
        "5186027266282590649_1661-h-1.htm.xhtml#chap01",
    )
    .expect("contents link should point at another spine item");

    assert_eq!(target.spine_index, 2);
    assert_eq!(target.fragment.as_deref(), Some("chap01"));
}

#[test]
fn ignores_external_links() {
    let docs = load_spine(crate::BOOK).expect("should open the bundled epub");

    assert_eq!(
        resolve_internal_link(&docs, 1, "https://www.gutenberg.org"),
        None,
    );
}
```

After the Rust tests pass, run `cargo clippy` too, because changing the `load_spine` return type
will touch the Dioxus `Reader` component.

### Minimal implementation sketch

1. Give each loaded item identity as well as content:

   ```rust
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub(crate) struct SpineDoc {
       pub(crate) href: String, // normalized EPUB path, e.g. "OEBPS/…h-1.htm.xhtml"
       pub(crate) xhtml: String,
   }
   ```

2. Change `load_spine` to return `Vec<SpineDoc>`. While iterating `epub.reader()`, keep the
   `manifest_entry().href()` (normalize leading `/` away if rbook returns one) and the rewritten
   XHTML from `read_str_with(&rewrite)`.

   ```rust
   pub(crate) fn load_spine(path: &str) -> Result<Vec<SpineDoc>, Box<dyn std::error::Error>> {
       let epub = Epub::open(path)?;

       let rewrite = EpubRewriteOptions::default().rewrite_paths(PathRewrite::prefix(
           format!("dioxus://index.html/{EPUB_ROUTE}/"),
       ));

       epub.reader()
           .map(|entry| {
               let entry = entry?;
               let manifest_entry = entry.manifest_entry();

               // `href()` returns an `rbook::Href`, not a `&str`. Use `.decode()`
               // (percent-decoded `Cow<str>`) so the stored path matches the
               // decoded link href in `resolve_internal_link`. Also drop any
               // leading "/" so "/OEBPS/…" == "OEBPS/…", the shape
               // `resolve_relative` produces.
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
   ```

   The `.map(...).collect()` over a `Vec<Result<_, _>>` still works because the
   closure returns `Result<SpineDoc, Box<dyn Error>>` and `collect()` turns
   `Iterator<Item = Result<T, E>>` into `Result<Vec<T>, E>` — the same shape Step 9
   landed, just yielding a struct instead of a `String`. Confirm `href()`'s exact
   spelling against your rbook (it may need the manifest entry, not the reader
   item) and whether it returns a leading slash — the `trim_start_matches('/')` is
   the cheap insurance either way.

3. Add the target type and resolver:

   ```rust
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub(crate) struct LinkTarget {
       pub(crate) spine_index: usize,
       pub(crate) fragment: Option<String>,
   }
   ```

   `resolve_internal_link(docs, current_index, href)` should:

   - return `None` for `http://`, `https://`, `mailto:`, etc.;
   - split the `#fragment` from the path;
   - treat `#only-a-fragment` as a link within the current spine item;
   - resolve a relative path against the current document's directory;
   - find the matching `SpineDoc.href` and return its index + fragment.

   In code (one way to satisfy those bullets — write your own and compare):

   ```rust
   pub(crate) fn resolve_internal_link(
       docs: &[SpineDoc],
       current_index: usize,
       href: &str,
   ) -> Option<LinkTarget> {
       // External targets aren't spine navigation — leave them be for now.
       if href.contains("://") || href.starts_with("mailto:") || href.starts_with("tel:") {
           return None;
       }

       // Split "path#fragment"; either half may be empty.
       let (path, fragment) = match href.split_once('#') {
           Some((path, frag)) => (path, Some(frag.to_string())),
           None => (href, None),
       };

       // A bare "#frag" stays inside the document we're already showing.
       if path.is_empty() {
           return Some(LinkTarget { spine_index: current_index, fragment });
       }

       // Decode percent-escapes so this matches the decoded `SpineDoc.href`
       // (rbook's `Href::decode` is the same `percent_decode_str` call).
       let path = percent_encoding::percent_decode_str(path).decode_utf8_lossy();

       // Resolve `path` against the *current document's* directory, then collapse
       // "." / ".." so it can be matched against a normalized SpineDoc.href.
       let base_dir = docs
           .get(current_index)?
           .href
           .rsplit_once('/')
           .map(|(dir, _file)| dir)
           .unwrap_or(""); // current doc sits at the zip root

       let resolved = resolve_relative(base_dir, &path);

       let spine_index = docs.iter().position(|doc| doc.href == resolved)?;
       Some(LinkTarget { spine_index, fragment })
   }

   /// Join `relative` onto `base_dir` and collapse `.`/`..` segments, URL-style.
   fn resolve_relative(base_dir: &str, relative: &str) -> String {
       // A leading "/" means "from the zip root", so it ignores base_dir.
       let base = if relative.starts_with('/') { "" } else { base_dir };

       let mut segments: Vec<&str> = Vec::new();
       for segment in base.split('/').chain(relative.split('/')) {
           match segment {
               "" | "." => {}                  // skip empty + current-dir markers
               ".." => { segments.pop(); }     // step up one directory
               other => segments.push(other),
           }
       }
       segments.join("/")
   }
   ```

   The one fact this leans on: `SpineDoc.href` must be stored in the **same
   normalized shape** `resolve_relative` produces — no leading `/`, no `..` left
   in it, and **percent-decoded** — or the `doc.href == resolved` match silently
   fails and every link resolves to `None`. That's why the loader uses `.decode()`
   and the resolver runs `percent_decode_str` on the incoming path: both sides
   have to agree on the spelling *and* the encoding. (For the Sherlock book the
   hrefs are plain ASCII, so the test passes either way — but decoding both sides
   is what makes links with spaces or Unicode resolve.)

4. Update existing call sites/tests from `doc.contains(...)` to `doc.xhtml.contains(...)`, and in
   `Reader` build pagination from `&docs[current()].xhtml`.

### Why it works

- The browser's default navigation cannot be the source of truth in this renderer, so Rust needs a
  small, testable model of "where would this EPUB link go?" before any UI event is involved.
- Carrying `href` next to `xhtml` preserves the context needed for relative links. Without the
  source document path, `chapter2.xhtml#x` is just a string; with it, you can resolve it exactly the
  way EPUB packaging expects.
- Returning `Option<LinkTarget>` keeps the policy explicit: internal spine links become reader
  navigation; external links are deliberately ignored for now instead of half-opening a browser.

### Scope note

This step does **not** intercept clicks inside the iframe yet and does not scroll to the fragment
once the target document loads. It only creates the pure Rust navigation target that the next UI
step can call. Step 11b will choose the iframe-to-Dioxus event channel and reset `page` to `0` when
an internal link changes spine item.

---

## Step 11b — wire internal-link clicks into spine navigation

> **Status:** done — committed in `4b895f6` (7 tests green; `cargo clippy --all-targets` clean;
> `dx serve` confirmed: clicking a TOC entry jumps to that chapter and resets to page 1). Landed as
> 11b-i + 11b-ii in one commit: the channel proved out *and* answered unknown #2 (rbook rewrites
> `<a href>` → absolute), so the resolver was switched to **Option A** and wired the same pass.
> Fragment scrolling stays deferred to a later step.

### The crux

Step 11a built the *destination* logic; 11b is the *transport* — getting a click that happens
**inside the iframe** out to Dioxus so it can call the resolver and move `current`. It's the hard
part of the phase, for reasons already recorded:

- The iframe is a **`data:application/xhtml+xml`** document with an **opaque origin**, rendered
  **script-free** (`sandbox="allow-same-origin"`, no `allow-scripts`).
- Letting the browser *follow* the link is the wrong primitive — internal links resolve toward
  `dioxus://…`, and **those iframe navigations are blocked by the desktop nav guard** (the Step 8
  finding). The click must be **intercepted, cancelled, and reported**, never followed.

A script-free sandbox can't intercept anything, so the mechanism is: **inject a trusted bridge
script** into the XHTML (the same pre-encode seam `inject_pagination_css` uses), **loosen the
sandbox to `allow-scripts`**, have the script `preventDefault()` + `postMessage` the href to the
parent, and run a Dioxus-side **`document::eval` listener** that forwards the href into a recv loop
that calls `resolve_internal_link` and sets `current` / resets `page`.

Two unknowns make this risky enough to split, spike-first:

1. **Does a click even escape?** Does `postMessage` from a sandboxed, opaque-origin `data:` iframe
   reach a parent `eval` listener? (Plausibly yes — `postMessage` is cross-origin by design — but
   unproven in *this* renderer.)
2. **What href shape arrives?** 11a's resolver was tested with a *hand-written relative* href.
   But `load_spine` runs `PathRewrite::prefix("dioxus://index.html/epub/")` — **if rbook rewrites
   `<a href>` too** (not just `img`/`link`), the click carries an absolute
   `dioxus://index.html/epub/OEBPS/…` URL the resolver won't match.
   **Answered by 11b-i (2026-06-30): yes, rbook rewrites `<a href>`.** The observed click
   carried `dioxus://index.html/epub/OEBPS/5186027266282590649_1661-h-1.htm.xhtml#chap01` — an
   absolute URL on the `dioxus://` scheme. So 11a's relative-resolution path never runs in the
   real app, and 11b-ii switches the resolver to **Option A** (strip the route prefix, match the
   remainder against `SpineDoc.href`) — see the re-derived 11b-ii below.

### Step plan

- **11b-i — prove the channel, observe the href.** Inject the bridge + `allow-scripts`, set up the
  eval listener, and just **display the raw href that arrives**. No resolver wiring. De-risks both
  unknowns; the *observed href shape* decides 11b-ii's resolver call.
- **11b-ii — switch the resolver to Option A, then wire it.** The observed href is absolute
  (`dioxus://index.html/epub/OEBPS/…`), so first **revise `resolve_internal_link`** to strip the
  `dioxus://index.html/{EPUB_ROUTE}/` prefix and match the remainder against `SpineDoc.href`
  directly (deleting the `://`-means-external check and the whole `resolve_relative` helper). Then
  feed the received href + `current()` into it; on `Some(target)` set `current` and reset `page` to
  `0`; drop the `#[allow(dead_code)]`.
- **Deferred (later step) — scroll to the fragment** once the target document loads (needs script
  *inside* the destination frame, which 11b-i's bridge makes possible).

---

### Step 11b-i — prove the iframe→Dioxus channel

> **Status:** done — folded into the 11b commit `4b895f6` (see the 11b status above). Both unknowns
> resolved: **#2** — the observed href was absolute (`dioxus://index.html/epub/OEBPS/…h-1.htm.xhtml#chap01`),
> confirming rbook rewrites `<a href>`; **#1** — `postMessage` does cross the opaque `data:` origin
> into the parent `document::eval` listener (navigation works). The debug `last_link` readout proved
> the channel, then was removed once 11b-ii navigated for real.

#### Runnable check (`dx serve` — eyeball)

Pure webview interop, so eyeball, not a unit test. Add a debug readout to the chrome (a
`use_signal(String)` rendered somewhere visible). Then under `dx serve`:

- Navigate to the **contents/TOC document** (spine 1 in the Sherlock book) — it has real in-book
  links.
- **Click a TOC entry.** The page must **not** navigate or blank out (the click is cancelled).
- The debug readout **shows the clicked link's href.** Note its exact shape — relative
  (`…1661-h-1.htm.xhtml#chap01`) or absolute (`dioxus://index.html/epub/OEBPS/…`). **That
  observation is the deliverable** — it decides 11b-ii's resolver call.
- `cargo clippy` clean.

If nothing appears, the channel itself is the bug (sandbox flags, XHTML well-formedness of the
injected script, or the eval listener) — which is exactly why this is isolated from the resolver.

#### Minimal implementation (sketch — you write it)

1. **Bridge-injection helper** in `src/epub.rs`, next to `inject_pagination_css` (same
   before-encode seam):

   ```rust
   pub(crate) fn inject_link_bridge(xhtml: &str) -> String {
       // Served as application/xhtml+xml → parsed as XML, so the script body must be
       // well-formed XML. CDATA-wrap it so a stray `<`/`>`/`&` in JS can't break the parse.
       let script = r#"<script type="text/javascript">
   //<![CDATA[
   document.addEventListener('click', function (e) {
       var a = e.target.closest && e.target.closest('a[href]');
       if (!a) return;
       e.preventDefault();
       window.parent.postMessage(
           { kind: 'ook-link', raw: a.getAttribute('href'), resolved: a.href },
           '*'
       );
   });
   //]]>
   </script>"#;
       xhtml.replacen("</head>", &format!("{script}</head>"), 1)
   }
   ```

   Sending **both** `raw` (attribute, possibly relative) and `resolved` (browser-absolute) is what
   answers unknown #2 — you see the difference.

2. **Inject before `to_xhtml_data_url`**, alongside the pagination injection:

   ```rust
   let paged = epub::inject_pagination_css(&docs[current()].xhtml, page());
   let bridged = epub::inject_link_bridge(&paged);
   let iframe_src = epub::to_xhtml_data_url(&bridged);
   ```

3. **Loosen the sandbox** so the injected script runs:

   ```rust
   sandbox: "allow-same-origin allow-scripts",
   ```

4. **Dioxus-side listener**, set up **once** so it persists (`use_future`, or `use_hook` + `spawn`):

   ```rust
   let mut last_link = use_signal(String::new); // the debug readout

   use_future(move || async move {
       let mut bridge = document::eval(
           r#"
           window.addEventListener('message', (e) => {
               if (e.data && e.data.kind === 'ook-link') {
                   dioxus.send(e.data.raw + "  |  " + e.data.resolved);
               }
           });
           "#,
       );
       loop {
           match bridge.recv::<String>().await {
               Ok(msg) => last_link.set(msg), // 11b-i: just observe
               Err(_) => break,
           }
       }
   });
   ```

   Confirm exact spellings against Dioxus 0.7 (`dioxus::document::eval`, the `dioxus.send` /
   `dioxus.recv` JS names, `eval.recv::<T>()`) — same hedge used for rbook calls. The *shape* (eval
   installs a `message` listener → `dioxus.send` → Rust `recv` loop) is the stable part.

#### Why it works

- **The injection seam is the data-URL seam.** Anything the XHTML parser must see — pagination CSS
  (Step 10), this bridge now — goes in **before** `to_xhtml_data_url` encodes the bytes. Reusing
  Step 10's mechanism exactly.
- **CDATA matters here and didn't in HTML.** `text/html` tolerates raw `<`/`&` in a `<script>`;
  `application/xhtml+xml` is XML, where an unescaped `i < 5` is a parse error that blanks the page.
  The `//<![CDATA[ … //]]>` wrapper lets JS hold XML-special characters. (None in this snippet, but
  the habit is the lesson.)
- **`allow-scripts` is a real loosening — name it.** It re-enables *all* scripts in the frame,
  including any the book ships, undoing the "book JS stays inert" property Step 7 chose. Gutenberg
  books carry none, so the risk is ~nil, but it's a deliberate trade. Hardening (strip book
  `<script>`s, inject only the bridge) is later. Note `allow-scripts` + `allow-same-origin` together
  is discouraged for *untrusted* content — acceptable for a semi-trusted local book, worth a comment.
- **`postMessage` crosses the opaque origin on purpose.** It's the one channel designed to work
  *regardless* of origin — why it beats any other way of reaching a `data:` document. The parent's
  `message` listener reads `event.data` structurally; no origin handshake needed child→parent.
- **Set the listener up once, or it leaks.** Installing `addEventListener` every render stacks
  duplicate listeners; `use_future`/`use_hook` running it once keeps exactly one bridge alive. The
  loop reads no reactive signal, so it won't restart — the property you'll protect in 11b-ii by
  reading `current` with `.peek()` (so the future doesn't re-subscribe and restart).

#### Scope note

11b-i **only observes** — proves the click escapes and reveals the href shape. It does **not** call
`resolve_internal_link`, move `current`, or scroll to the fragment. Wiring the resolver is
**11b-ii**; fragment scrolling is a later step (needs script *inside* the destination frame, which
this bridge now enables).

> **Alternative channel if `postMessage` is blocked:** the script could
> `fetch("dioxus://index.html/epub/__nav/…")` — a *subresource* request that bypasses the nav guard
> and reaches the asset handler (the same asymmetry that let Step 7's images load). But bridging the
> handler (a `'static` `Rc<Epub>` closure off the UI thread) back into a signal needs its own
> channel, so `postMessage`+`eval` is the idiomatic default. Back-pocket option for debugging, not
> a thing to build preemptively.

---

### Step 11b-ii — switch the resolver to Option A, then wire it

> **Status:** done — committed in `4b895f6` with 11b-i (7 tests green; clippy clean). The resolver
> dropped the `://`-means-external check and `resolve_relative` for a single
> `strip_prefix("dioxus://index.html/{EPUB_ROUTE}/")?`; the `use_future` listener calls it with
> `*current.peek()` and, on `Some(target)`, sets `current` + resets `page`. Wiring lessons recorded
> below: the `FnMut` future needs `docs` cloned in its body, and the `current` read must be a
> `let idx = *current.peek();` snapshot *before* the `if let` so its borrow guard is dropped before
> `current.set()` (else E0502 — a scrutinee temporary lives for the whole `if let` body).

#### The crux

11a built `resolve_internal_link` to take a *relative* href and walk it against the current
document's directory (`base_dir` + `resolve_relative`). 11b-i proved that's the wrong input model:
rbook rewrites `<a href>` the same way it rewrites `<img>`/`<link>`, so the click delivers a
**fully-resolved absolute URL** — `dioxus://index.html/epub/OEBPS/…h-1.htm.xhtml#chap01`.

rbook already did the relative-path resolution at load time. So the resolver's job collapses to:
**strip the route prefix off the absolute URL and match the remainder against `SpineDoc.href`.**
The prefix is the same contract string `load_spine` rewrites with —
`dioxus://index.html/{EPUB_ROUTE}/` — so the stripped tail (`OEBPS/…h-1.htm.xhtml`) is already in
the exact normalized shape `SpineDoc.href` stores. `resolve_relative` and the `base_dir` lookup
become dead code; the `href.contains("://")` external check inverts into "must carry our prefix."

This is **Option A**, and it makes the resolver *smaller*: the rewrite that complicated the input is
also what removes the work.

#### Runnable check (`cargo test`)

Revise the two existing resolver tests in `src/epub.rs` to feed the **absolute** href the app
actually produces (the relative-input tests describe a path the real renderer never takes):

```rust
#[test]
fn resolves_contents_link_to_spine_doc_and_fragment() {
    let docs = load_spine(crate::BOOK).expect("should open the bundled epub");

    // The shape rbook actually rewrites an in-book <a href> to (confirmed in 11b-i),
    // not the hand-written relative form 11a originally tested.
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
fn ignores_external_links() {
    let docs = load_spine(crate::BOOK).expect("should open the bundled epub");

    // No epub-route prefix → not spine navigation. (Still true; no change needed here,
    // but it now passes via the prefix check rather than the `://` check.)
    assert_eq!(
        resolve_internal_link(&docs, 1, "https://www.gutenberg.org"),
        None,
    );
}
```

Red first: the contents-link assertion fails against today's resolver, because the absolute URL
trips the `href.contains("://")` guard and returns `None`. That failure *is* the proof the old
input model was wrong.

> **Confirm the `OEBPS/` segment against your book.** The expected stripped tail must equal
> `docs[2].href`. If `manifest_entry().href()` stores a different folder (or none), adjust the test
> URL to match what `load_spine` actually produced — the existing `loads_spine_in_reading_order`
> test plus a one-off `dbg!(&docs[2].href)` tells you the truth rather than guessing.

#### Minimal implementation sketch (you write it)

Revise `resolve_internal_link` — split the fragment, keep the bare-`#frag` guard, then strip the
prefix instead of resolving relatively:

```rust
pub(crate) fn resolve_internal_link(
    docs: &[SpineDoc],
    current_index: usize,
    href: &str,
) -> Option<LinkTarget> {
    // Split "url#fragment"; either half may be empty.
    let (path, fragment) = match href.split_once('#') {
        Some((path, frag)) => (path, Some(frag.to_string())),
        None => (href, None),
    };

    // A bare "#frag" stays inside the document we're already showing. (rbook *may*
    // rewrite even these to an absolute current-doc URL — if so this branch is just
    // insurance and the prefix path below resolves it the same way.)
    if path.is_empty() {
        return Some(LinkTarget { spine_index: current_index, fragment });
    }

    // rbook rewrote in-book links to absolute `dioxus://index.html/epub/<zip-path>`.
    // Anything without that prefix (https:, mailto:, tel:) isn't spine navigation.
    // The stripped tail is already in SpineDoc.href's normalized shape.
    let prefix = format!("dioxus://index.html/{EPUB_ROUTE}/");
    let zip_path = path.strip_prefix(&prefix)?;

    // Decode percent-escapes so this matches the decoded SpineDoc.href.
    let zip_path = percent_encoding::percent_decode_str(zip_path).decode_utf8_lossy();

    let spine_index = docs.iter().position(|doc| doc.href == zip_path)?;
    Some(LinkTarget { spine_index, fragment })
}
```

Then **delete `resolve_relative`** (now unused) and the `base_dir` lookup. Once `cargo test` is
green, do the actual wiring from 11b-i's recv loop: replace `last_link.set(msg)` with a call into
`resolve_internal_link(&docs, current(), &raw)`, and on `Some(target)` `current.set(target.spine_index)`
+ `page.set(0)`. Drop the `#[allow(dead_code)]` on the resolver. Send only `raw` over the bridge now
that `resolved` has served its diagnostic purpose (or keep both — your call).

#### Why it works

- **The rewrite is the resolver.** rbook resolved every relative link against its document's
  location at load time; stripping the known prefix recovers a key that matches `SpineDoc.href`
  one-to-one. Re-deriving the relative path in Rust would just redo work rbook already did — and
  `resolve_relative` was the part most likely to get a `..`/`.` edge case wrong, so deleting it
  removes a whole class of bug.
- **The external-link check inverts cleanly.** "Has our prefix?" is a stricter, more honest test
  than "contains `://`": a `dioxus://…/epub/…` URL *does* contain `://` yet is internal, so the old
  guard was actively wrong for the real input. `strip_prefix(&prefix)?` is both the classifier and
  the extractor in one line.
- **`current_index` survives for one reason.** It's no longer needed to resolve paths (they're
  absolute), only to answer a bare `#fragment` link that means "this document." Keeping the param
  costs nothing and keeps that case correct.

#### Scope note

This still doesn't **scroll to the fragment** once the destination loads — that needs a script
*inside* the target frame and stays deferred (the 11b-i bridge makes it possible later). It also
leaves external links inert (returned `None`), same policy as 11a. The `resolve_relative` deletion
is a genuine simplification, not behavior loss: nothing in the real app ever fed it a relative
href.

---

## Step 11c — scroll to the link's fragment

> **Status:** done — committed in `3b5aee5` (8 tests green). Deferred from 11b — the resolver
> already returns `LinkTarget.fragment`; 11b just dropped it after navigating. This step *uses* it.

### The crux

11b navigates to the target spine document and resets to **page 0**. But a `#fragment` like
`#chap02` can live *mid-document* — in a later CSS column — so page 0 isn't where the anchor is.
Two things make "scroll to it" harder than it sounds in this renderer:

- **The iframe loads asynchronously.** Setting `current` swaps the iframe's `data:` URL; the new
  document isn't parsed *yet* when the listener returns, so you can't measure the anchor
  synchronously after `current.set(...)`. The measurement has to happen **on load, inside the
  frame**.
- **There's no scrolling to hijack.** Step 10's layout is `html { overflow: hidden }` + a
  `body` laid out in viewport-wide columns moved by `translateX`. So the browser's native
  "scroll to `#frag`" does nothing — there's no scroll container. "Going to the anchor" means
  **picking the page (column) the anchor sits in**, not scrolling.

The unlock is the seam you already built twice: **inject a script before encoding.** Inject an
on-load script that finds the element, computes which column it's in (`offsetLeft / innerWidth`),
and reports that page back through the **same `postMessage` → `document::eval` bridge**. Dioxus
sets `page`. The one trap is a **reload loop** — setting `page` re-encodes the `data:` URL and
reloads the frame, which would re-run the measure script — so the pending fragment is **consumed
once** and cleared, and the reload carries no scroll script.

### Runnable check

Two checks, same split as Steps 10 and 11b.

#### 1. Pure Rust helper (`cargo test`)

`inject_fragment_scroll` is another pure string injector like `inject_pagination_css` /
`inject_link_bridge`, so it gets the same kind of test:

```rust
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
```

Red first: no `inject_fragment_scroll` yet.

#### 2. UI wiring (`dx serve` + `cargo clippy`)

Under `dx serve`:

- Open the contents/TOC document and click a link whose anchor is **mid-document** (not a
  chapter-top). The reader lands on the **page showing that anchor**, not page 0.
- Page +/- still work from there; chapter Next/Prev still reset to page 0.
- Clicking a link whose anchor is at the document top still lands on page 0 (unchanged).
- `cargo clippy` clean.

> **Pick the test link deliberately.** If every TOC anchor in this book sits at the top of its
> document, page 0 already shows it and this step is a no-op to the eye — confirm with one
> `dbg!`/`console.log` of the computed page that the measurement runs, and note in the commit
> that mid-document anchors are the case it really serves.

### Minimal implementation sketch (you write it)

1. **The injector**, next to the others in `src/epub.rs`:

   ```rust
   pub(crate) fn inject_fragment_scroll(xhtml: &str, fragment: &str) -> String {
       // Runs once the frame's document has parsed, so the element exists and the
       // column layout is settled. offsetLeft is the element's *pre-transform* layout
       // x, so dividing by the viewport width gives its column index regardless of the
       // current translateX. Reports the page back over the same bridge as a click.
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
       xhtml.replacen("</head>", &format!("{script}</head>"), 1)
   }
   ```

2. **Remember the pending fragment** in `Reader`:

   ```rust
   let mut pending_fragment = use_signal(|| None::<String>);
   ```

3. **Inject it only when one is pending**, after the link bridge, before encoding:

   ```rust
   let bridged = epub::inject_link_bridge(&paged_doc);
   let prepared = match pending_fragment() {
       Some(frag) => epub::inject_fragment_scroll(&bridged, &frag),
       None => bridged,
   };
   let iframe_src = epub::to_xhtml_data_url(&prepared);
   ```

4. **Carry the fragment through the click branch, and handle the new report.** The bridge now
   speaks two message kinds, so tag them in the eval listener and branch in Rust:

   ```js
   // in the document::eval listener:
   window.addEventListener('message', (e) => {
       if (!e.data) return;
       if (e.data.kind === 'ook-link')   dioxus.send("link:" + e.data.raw);
       if (e.data.kind === 'ook-scroll') dioxus.send("scroll:" + e.data.page);
   });
   ```

   ```rust
   while let Ok(msg) = bridge.recv::<String>().await {
       if let Some(href) = msg.strip_prefix("link:") {
           let idx = *current.peek();
           if let Some(target) = epub::resolve_internal_link(&docs, idx, href) {
               current.set(target.spine_index);
               page.set(0);
               pending_fragment.set(target.fragment); // may be None — that's fine
           }
       } else if let Some(p) = msg.strip_prefix("scroll:") {
           if let Ok(p) = p.parse::<usize>() {
               page.set(p);
               pending_fragment.set(None); // consume once → reload carries no scroll script
           }
       }
   }
   ```

### Why it works

- **On-load is the only correct moment.** The element doesn't exist until the new `data:`
  document parses; `window.addEventListener('load', …)` fires after that, so `getElementById`
  finds it and the column layout is final. Measuring right after `current.set()` in Rust would
  race the load and find nothing.
- **`offsetLeft / innerWidth` is the page.** In the multicolumn body each column is one viewport
  wide, so an element's layout x divided by the viewport width *is* its column index. `offsetLeft`
  is the pre-transform layout position, so it's correct no matter what `--ook-page`/`translateX`
  is showing when the script runs.
- **Consume-once breaks the reload loop.** `page.set(p)` re-encodes the `data:` URL (it carries
  `--ook-page`), so the frame reloads. Clearing `pending_fragment` in the same handler means the
  reload's document is built *without* the scroll script, so it doesn't measure-and-report again.
  Without that clear, every fragment jump would ping-pong forever.
- **Tagging keeps one channel.** Rather than a second `eval` listener, the existing bridge gains a
  `"scroll:"` message alongside `"link:"`; `strip_prefix` is the same classify-and-extract move the
  resolver uses. One transport, two messages.

### Scope note

A deliberately rough first cut, matching the Step 10 spike it builds on:

- **Page granularity, not pixel-perfect.** It lands on the right *page*; it does not fine-position
  vertically within a column. Fine for chapter/section anchors.
- **A brief page-0 flash** before the report arrives and `page` jumps. Acceptable for the spike;
  smoothing it (compute before first paint) is later.
- **No deep-link on first load** — only fragments arriving via an in-app click are handled, not a
  fragment present when the book first opens.
- **The fragment id is interpolated raw into JS.** Gutenberg ids are plain (`chap02`); an id
  containing a `"` would break the string. Escaping is deferred — note it, don't solve it now.
- **Does not** revisit external links or change the resolver; it only consumes the `fragment` the
  resolver already returns.

---

## Step 12 — make the sample `.epub` fixture intentional and documented

> **Status:** done — committed in `56f2af3` (9 tests green).

### The crux

The phase checklist calls this "bundle a small DRM-free sample `.epub`," but the twist is that the
book is **already committed** — `git ls-files` shows
`book/The Adventures of Sherlock Holmes by Arthur Conan Doyle.epub` is tracked. So nothing needs
"adding." What's ad-hoc is everything *around* the file:

- **The path is a brittle, human title with spaces** hard-coded in `src/main.rs`
  (`const BOOK: &str = "book/The Adventures of Sherlock Holmes by Arthur Conan Doyle.epub"`) and
  echoed in four `src/epub.rs` tests via `crate::BOOK`.
- **It sits next to a gitignored scratch dir** (`book/unzipped/`), so "what in `book/` is the real
  fixture" isn't obvious to a fresh reader.
- **Nothing records its provenance or licence.** The whole phase asserts "DRM-free EPUBs only," yet
  nothing *documents* that this file is public-domain, where it came from, or why it was chosen —
  and the tests silently depend on its specific shape (15-item spine, "A Scandal in Bohemia" at
  spine index 2, `#chap01` there, a JPEG/PNG cover).

So this step is deliberately **not** a feature step and **not** much Rust — it's *intentional-izing*
a fixture: give it a stable name that encodes its identity, write down its provenance + licence +
why-this-book, make the load path robust to the working directory, and pin "the fixture is actually
bundled" with a test so a future gitignore slip fails loudly instead of as a cryptic `Epub::open`
error. **Keep the Sherlock Holmes book** — it already exercises the entire render path (cover, CSS,
images, TOC, internal links, 15 documents) and the whole suite is calibrated to it; swapping books
would churn every test for no gain.

### Runnable check

Two checks, and be honest about what each is:

#### 1. New pure-Rust test (`cargo test`) — pin "the fixture is bundled"

The one genuinely new invariant this step adds is *the fixture exists at its committed path*. Today
nothing guards that: if `book/` were gitignored or the file moved, every test would fail deep inside
`Epub::open` with a vague error. A tiny existence test makes the failure name the real cause. Add it
to `src/epub.rs`'s test module:

```rust
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
```

Red first: it fails the instant `BOOK` points at a path that isn't there — which is exactly what you
want it to catch after the rename below, until you `git mv` the file to match.

#### 2. Safety net (`cargo test` + `cargo clippy`) — the rename changes nothing else

Renaming the file + updating `BOOK` is a path change, not a behavior change, so the **entire existing
suite must stay green, identically, before and after** — `loads_spine_in_reading_order` (15 docs,
"A Scandal in Bohemia"), `reads_cover_image_bytes`, the resolver tests, all of it. That green suite
*is* the proof the fixture is still intact under its new name. `cargo clippy` stays clean.

### Minimal implementation sketch (you write it)

There's very little Rust here; the substance is file moves + a doc + one constant.

1. **Rename the file to a stable, identity-encoding name** with `git mv` (keeps history):

   ```sh
   git mv "book/The Adventures of Sherlock Holmes by Arthur Conan Doyle.epub" \
          "book/pg1661-adventures-of-sherlock-holmes.epub"
   ```

   `pg1661` ties it to its source (Project Gutenberg ebook #1661) and drops the spaces that make the
   path awkward to type and quote.

2. **Point `BOOK` at the new name — and make it robust to the working directory.** Today `BOOK` is a
   path *relative to the crate root* that only resolves because `cargo` happens to run from there
   (flagged all the way back in Step 1). Anchor it to the crate root explicitly with the compile-time
   `CARGO_MANIFEST_DIR` env var so it resolves no matter the CWD:

   ```rust
   // src/main.rs
   pub(crate) const BOOK: &str =
       concat!(env!("CARGO_MANIFEST_DIR"), "/book/pg1661-adventures-of-sherlock-holmes.epub");
   ```

   All the `crate::BOOK` call sites in `src/epub.rs` and `src/main.rs` pick up the new value for
   free — that's the payoff of its already being one shared `const`.

3. **Write `book/README.md`** — the actual deliverable of "documented." Record what a future you (or
   a contributor) needs to trust and not accidentally break:

   - **Source:** Project Gutenberg ebook #1661, *The Adventures of Sherlock Holmes* by Arthur Conan
     Doyle (link the Gutenberg page).
   - **Licence:** public domain in the US (Project Gutenberg terms); safe to commit and redistribute —
     this is what satisfies the phase's "DRM-free only" constraint, made explicit.
   - **Why this book:** it exercises the *whole* render path in one file — a cover image, the book's
     own CSS, inline images, a real table-of-contents with in-book links, and a 15-document spine —
     so the render/paging/link tests have something real to bite on.
   - **Invariants the tests depend on:** spine length 15; "A Scandal in Bohemia" is spine index 2 and
     carries `#chap01`; the cover is a JPEG/PNG. Changing the file breaks these tests on purpose.
   - **Note `book/unzipped/` is gitignored** scratch (an unpacked copy for eyeballing), *not* the
     fixture.

4. **(Optional) confirm `.gitignore` can't swallow the fixture.** The ignore file lists
   `book/unzipped/` (good — scratch only) and `*.db`; nothing there matches the `.epub`, so the file
   stays tracked. Worth an eyeball, not an edit.

### Why it works

- **A name that encodes identity beats a title with spaces.** `pg1661-…` tells the next reader
  *what this is and where it's from* at a glance, and a shell- and code-friendly path removes the
  quoting friction the current name imposes on every `git mv`, test, and `dx serve`.
- **`concat!(env!("CARGO_MANIFEST_DIR"), …)` is resolved at compile time**, so `BOOK` becomes an
  absolute path baked into the binary — it no longer silently depends on *where* you launched
  `cargo`/`dx`. `env!` reads the var at build time (it's set by Cargo); `concat!` glues the string
  literals into one `&'static str`, so `BOOK` is still a plain `const` with no runtime cost. That's
  the small, real Rust lesson riding inside this housekeeping step.
- **The existence test converts a silent dependency into a named one.** "Fixture must be present" is
  currently an *implicit* precondition of a dozen tests; making it its own assertion means a missing
  or gitignored file fails with a message that says *why*, instead of a confusing parse error three
  layers down.
- **The README is the difference between "a book happens to be here" and "this is the fixture."** It
  turns tribal knowledge (why this book, is it legal to ship, what the tests assume) into something a
  contributor can read — which is the whole point of the step's word "documented."

### Scope note

- **This is not the phase's final refactor** — that's Step 13, and it runs *after* this, once the
  fixture is intentional. This step deliberately touches only the fixture (name, path, docs, one
  existence test); it does not rename `SpineList`/`Reader`, tidy the render path, or revisit the
  `.body(…).unwrap()`/`#![allow(non_snake_case)]` items parked in Step 9's deferred list. Resist
  folding those in here — keep the diff about the fixture so the review step has a clean baseline.
- **No second fixture, no test-runner plumbing.** A single real book is enough for this phase; a
  matrix of tiny synthetic EPUBs (empty spine, missing cover, malformed href) is a *later* concern
  when error-handling gets its own steps — note it, don't build it now.
- **The path is still hard-coded to one bundled book.** Opening an *arbitrary* user-chosen `.epub`
  (a file picker, a library) is Milestone 2's later features, not this phase — `BOOK` staying a
  `const` is correct for now.

---

## Step 13 — review & refactor the finished EPUB rendering phase

> **Status:** planned — the phase-ending review-and-refactor pass. Baseline before starting:
> **9 tests green, `cargo clippy --all-targets` clean.**

### The discipline (this is a review, not a feature)

The feature arc is done: the reader opens the book, renders each chapter faithfully as XHTML,
paginates in columns, and follows internal links to the right page. Steps 4–8 got their own
scoped review (Step 9); this is the **whole-phase** pass the repo convention ends every phase
with — make the landed code *idiomatic and well-organized*, not just working.

**The safety net is the spec, not the target.** The 9 tests + clippy are green now; they must be
green *identically* after each change. A refactor that needs a test to change isn't a refactor —
it's a feature, and it doesn't belong in this step. New tests are fair game only to lock down
behavior that's currently implicit (e.g. the `</head>`-insertion helper in item **B**). Run
`cargo test && cargo clippy --all-targets` before and after every item.

The hard rule still holds: **I propose, you edit.** Exact before/after snippets are fine in a
review step — but the diff lands in `src/` by your hand.

### The punch-list (highest-leverage first)

#### A. Centralize the `dioxus://index.html/{EPUB_ROUTE}/` prefix — the one real drift risk

The absolute-URL prefix is computed in **two** places that *must* agree, or internal links
silently resolve to `None`:

- `src/epub.rs:42` — `load_spine` rewrites in-book URLs with
  `PathRewrite::prefix(format!("dioxus://index.html/{EPUB_ROUTE}/"))`.
- `src/epub.rs:123` — `resolve_internal_link` strips
  `format!("dioxus://index.html/{EPUB_ROUTE}/")` back off.

These are the *same contract* (the write side and the read side of the link round-trip) written
twice. Step 9 already lifted the bare `/epub` route into `EPUB_ROUTE`; this is the remaining
half — the `dioxus://index.html/` host literal. Because the whole value is known at compile
time, make it a `&'static str` **const** (not a runtime `format!` — that would allocate a fresh
`String` in `resolve_internal_link` on every link click), next to `EPUB_ROUTE`:

```rust
// src/epub.rs, near EPUB_ROUTE
pub(crate) const EPUB_ROUTE: &str = "epub";
pub(crate) const EPUB_URL_PREFIX: &str = "dioxus://index.html/epub/"; // must embed EPUB_ROUTE
```

Then both sites use the const directly: `resolve_internal_link` does
`path.strip_prefix(EPUB_URL_PREFIX)?` (no allocation now), and `load_spine` hands
`EPUB_URL_PREFIX` to `PathRewrite::prefix`.

Two plain adjacent consts keep this dead-simple and readable — but they each still spell `epub`,
so guard them from drifting apart with a **dedicated test**. That's what buys back the
single-source safety without reaching for a macro, and it's a legitimate new test for a refactor
step because it locks down a currently-*implicit* invariant (the comment "must embed
EPUB_ROUTE") rather than changing behavior:

```rust
#[test]
fn url_prefix_embeds_the_route() {
    // The asset handler registers under EPUB_ROUTE; the rewrite/resolve prefix must carry that
    // same route as a path segment, or in-book links silently resolve to None. This fails the
    // instant the two literals drift apart.
    assert!(
        EPUB_URL_PREFIX.contains(&format!("/{EPUB_ROUTE}/")),
        "EPUB_URL_PREFIX ({EPUB_URL_PREFIX}) must contain the /{EPUB_ROUTE}/ segment",
    );
}
```

**Why:** the write and read sides drifting apart is the single bug that would break *all* link
navigation at once while every unit test using a hard-coded URL still passed. The two consts make
the value obvious and allocation-free; the drift-guard test converts "must embed EPUB_ROUTE" from
a hopeful comment into an enforced invariant. It also puts the macOS-specific
`dioxus://index.html/` host (a known platform coupling, per Step 8) in one documented spot
instead of scattered string literals.

> **Note — two new tests in a refactor step.** This drift-guard and item B's
> `insert_before_head_close` no-op test both *lock down implicit behavior* rather than assert new
> behavior, so they're consistent with this step's "safety net is the spec" rule. Everything else
> here leaves the assertion set untouched.

#### B. Collapse the three `inject_*` helpers' shared `</head>` insertion

`inject_pagination_css`, `inject_link_bridge`, and `inject_fragment_scroll` each end with the
identical line (`src/epub.rs:85`, `:103`, `:151`):

```rust
xhtml.replacen("</head>", &format!("{snippet}</head>"), 1)
```

Three copies of the same "insert this before `</head>`" move — and three copies of the same
latent fragility (if a document has no `</head>`, or odd casing, the snippet is silently
dropped). Extract one helper:

```rust
/// Insert `snippet` immediately before the first `</head>`. If there is none, the
/// document is returned unchanged — the injected feature just no-ops for that doc.
fn insert_before_head_close(xhtml: &str, snippet: &str) -> String {
    xhtml.replacen("</head>", &format!("{snippet}</head>"), 1)
}
```

and have each injector build its snippet, then call it. **Why:** the three injectors are the
data-URL injection seam this whole renderer is built on; naming the shared mechanism once means
the "what if `</head>` is missing" question has *one* answer to harden later instead of three to
keep in sync. This is the one place a **new test** is justified — it locks down the currently
implicit no-op-on-missing-`</head>` behavior:

```rust
#[test]
fn insert_before_head_close_is_a_noop_without_a_head() {
    let out = insert_before_head_close("<html><body>x</body></html>", "<style/>");
    assert_eq!(out, "<html><body>x</body></html>");
}
```

#### C. Lift the iframe-src injection pipeline out of `Reader` into `epub.rs`

`Reader` (`src/main.rs:66–73`) inlines the whole document-preparation pipeline:

```rust
let current_doc = &docs[current()];
let paged_doc = epub::inject_pagination_css(&current_doc.xhtml, page());
let bridged = epub::inject_link_bridge(&paged_doc);
let prepared = match pending_fragment() {
    Some(frag) => epub::inject_fragment_scroll(&bridged, &frag),
    None => bridged,
};
let iframe_src = epub::to_xhtml_data_url(&prepared);
```

That's five lines of **EPUB/render-document logic** (the pagination → bridge → fragment → encode
*order* is a domain fact) living inside a UI component. Pull it into one named function in
`epub.rs`:

```rust
pub(crate) fn render_document_url(
    doc: &SpineDoc,
    page: usize,
    fragment: Option<&str>,
) -> String {
    let paged = inject_pagination_css(&doc.xhtml, page);
    let bridged = inject_link_bridge(&paged);
    let prepared = match fragment {
        Some(frag) => inject_fragment_scroll(&bridged, frag),
        None => bridged,
    };
    to_xhtml_data_url(&prepared)
}
```

so the component reads `let iframe_src = epub::render_document_url(current_doc, page(), pending_fragment().as_deref());`.
**Why:** the component should be about *reader state* (which chapter, which page, is a scroll
pending) — how a document gets turned into a `data:` URL is domain logic, and the injection
*order* matters (the parser must see pagination + scripts before it's encoded). Naming it once
makes that order a single reviewable, testable thing, and shrinks `Reader` to the state it
actually owns. It also makes the ordering assertion unit-testable if you ever want it (all four
injections present, in order) — though don't add that test unless it earns its keep.

#### D. Drop the dead `resolved` payload from the link bridge

`inject_link_bridge` still posts both fields (`src/epub.rs:96`):

```js
window.parent.postMessage(
    { kind: 'ook-link', raw: a.getAttribute('href'), resolved: a.href },
    '*'
);
```

but the Rust side only ever consumes `raw` (`src/main.rs:83`: `dioxus.send("link:" + e.data.raw)`).
`resolved` was 11b-i's *diagnostic* — it existed to reveal the absolute-URL shape, which it did,
and 11b-ii's finding is now baked into the resolver. It's dead payload. Delete `resolved:
a.href,` from the message. **Why:** dead scaffolding tells the next reader "this matters" when it
doesn't — and here it actively misleads, implying the Rust side chooses between raw and resolved
when it settled that question three steps ago. (Its removal changes no test — the bridge is
eyeball-verified.)

#### E. Land Step 9's three deferred polish items — this is the review they were deferred to

Step 9 explicitly parked these "optional polish, no churn risk" items for the phase's *final*
review. This is it:

1. **`.body(…).unwrap()` → `.expect(…)`** (`src/main.rs:36`, `:40`). `Response::builder().body(…)`
   only errors on an invalid header/status set earlier — impossible here — so an `.expect`
   documents *why* it can't fail: `.expect("response with a valid content-type header")` and
   `.expect("empty 404 body is always valid")`. **Why:** a bare `unwrap()` says "I didn't think
   about this"; an `expect` says "I did, here's the invariant."
2. **Crate-level `#![allow(non_snake_case)]` → scoped `#[expect]`** (`src/main.rs:1`). The blanket
   allow silences the *whole crate* to accommodate two PascalCase Dioxus components (`App`,
   `Reader`). Narrow it to the two spots that need it:
   ```rust
   #[expect(non_snake_case, reason = "Dioxus components are PascalCase by convention")]
   #[component]
   fn App() -> Element { … }
   ```
   **Why:** `#[expect]` *fails* if the lint stops firing (so a stray future `snake_case` fn isn't
   silently un-linted), and scoping it means the rest of the crate keeps normal naming
   enforcement instead of a crate-wide blind spot. *(Confirm `#[expect]` sits cleanly with
   `#[component]`; if the macro expansion fights it, fall back to a scoped
   `#[allow(non_snake_case)]` on each `fn` with the same reason in a comment — the point is
   *scoped*, not crate-wide.)*
3. **Wire or delete `MAIN_CSS`** — it's now genuinely *used* (`src/main.rs:12`, `:51–54` mount it
   as a stylesheet `document::Link`), so Step 9's "dead scaffolding" note is already resolved.
   Just confirm `assets/main.css` isn't empty/stale; nothing to do if it's real.

### Optional — cut any of these to keep the pass light

- **Chapter/page index conventions disagree.** The chrome shows `"Chapter {current()}"` (0-indexed
  — the cover reads "Chapter 0", `src/main.rs:131`) but `"Page {page() + 1}"` (1-indexed,
  `:151`). Pick one for the display (1-indexed reads more naturally to a human;
  `"Chapter {current() + 1}"`). Display-only — no test asserts this text — but confirm under
  `dx serve`. *Watch:* this is a UI-label tweak, not renderer behavior; if it starts to feel like
  "what does chapter 1 mean for a cover," that's a real question for a later navigation/TOC step,
  not this refactor.
- **The two nav rows are near-identical** "Prev / label / Next" triples (`src/main.rs:120–158`).
  A tiny `NavRow` component (or a helper returning the `rsx!`) would deduplicate them — but two
  instances is borderline, and the click handlers differ (chapter resets `page`, page clamps).
  Extract only if it reads cleaner to you; skip it otherwise.
- **Inline `style:` strings** scattered through the rsx could move to `assets/main.css` classes —
  but styling the app chrome is really **Phase 4 (theming)** territory, so leave the inline styles
  for now rather than half-doing a theming refactor here. Noted, deferred on purpose.

### Commit shape

A single `refactor:` commit is natural since none of this changes behavior (the 9 tests stay
green throughout). If you'd rather, split the *pure-Rust* moves (A–C, E) from the *JS/bridge*
cleanup (D) — but one `refactor:` reads fine here. Per repo convention: no co-author /
AI-attribution trailer. `lbb:commit` handles it and writes the done-status marker back here.

### Why this closes the phase

After Step 13 the render path has: one definition of the URL contract (A), one document-injection
mechanism (B) and one document-preparation pipeline (C), no diagnostic dead weight (D), and the
error-handling / lint polish Step 9 deferred (E) — so the code a future you re-reads is
*idiomatic and organized*, and the phase can be marked ✅ done in the phase doc and the roadmap.
