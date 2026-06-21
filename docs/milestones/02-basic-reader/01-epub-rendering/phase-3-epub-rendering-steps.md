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
9. ⬜ **Review & refactor the rendering arc** — the review-and-refactor convention, scoped here
   to the render path (Steps 4–8), not the whole phase (pagination / internal links / sample
   epub still follow). Reconcile the churn Step 8 left (the rewrite machinery is **repurposed**
   if 8b rewrites URLs, or genuinely dead if 8b uses `<base>`), make names honest for whatever
   `load_spine` now returns (content / `data:` URLs, not `paths`), and (optional) lift the EPUB
   logic out of `main.rs`. Safety net: the post-Step-8 suite stays green + clippy clean, no
   behavior change. *(dead-code vs repurpose, naming honesty, module boundaries)*

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

> **Status:** ⬜ planned. The review-and-refactor convention applied to the **render path
> (Steps 4–8)**, run right after Step 8 lands. **Scoped to the rendering arc, not the whole
> phase** — pagination, internal-link navigation, and bundling a sample epub still follow, and
> the phase gets its *true* final review when they land. Reviewing the arc now, while the Step 8
> detour (served XHTML → `data:` URLs) is fresh and has left churn behind, is the point.

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
