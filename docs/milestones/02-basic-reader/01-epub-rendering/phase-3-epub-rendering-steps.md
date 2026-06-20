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
3. **Turn pages** — a `use_signal` index; Next/Prev mutate it, clamped to `0..docs.len()`.
   Eyeball: page through all 15 items. *(signals, event handlers, clamping)*

### Faithful-styling arc — load the book's own images & CSS (Steps 4–7)

The deferred ADR-0002 unlock, now pulled forward on purpose and split smallest-first. The
shape: **serve** the zip's bytes at a URL the webview can fetch (custom protocol), then
**point** each document's relative URLs at it inside an isolated iframe. Testable pure-Rust
seams first (Steps 4, 6), webview wiring eyeballed (Steps 5, 7).

4. **Read the cover image's bytes out of the zip** — pure Rust, `cargo test` against the real
   book. *(rbook `manifest().cover_image()` / `read_bytes`, media-type, magic numbers)*
5. **Register the `use_asset_handler("epub", …)` custom protocol** — map a request path →
   resource bytes → `responder.respond(Response …)` with the right `content-type`. Eyeball
   via devtools / a tiny `<img>`. *(`use_asset_handler`, `http::Response`, the `wry://` vs
   `http://wry.` scheme split across platforms)*
6. **Rewrite a spine doc's OPF-relative URLs to the `epub://` protocol** — pure Rust,
   `cargo test` on a sample XHTML string (or inject a `<base href>`). *(string rewriting,
   OPF-relative paths)*
7. **Render `docs[current]` in a sandboxed `<iframe>`** wired to the protocol — swap the
   all-docs `dangerous_inner_html` column for one isolated item. Eyeball: the cover renders,
   the book's CSS applies, app styles don't leak. *(iframe `srcdoc` + `sandbox`)*

> **Dependency.** Step 7 renders `docs[current]`, so it needs the `current` signal from
> **Step 3**. Steps 4–6 don't — they can be written now, in any order, before or alongside
> Step 3. Do Step 3 before Step 7.

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

## Step 4 — read the cover image's bytes out of the zip

> **Status:** planned — first step of the faithful-styling arc.

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

When this is green, the next concrete step is **Step 5 (register the protocol)** — want me to
lay that one out in full? (Or, if you'd rather finish **Step 3 / Turn pages** first since it's
the in-flight step and Step 7 depends on it, say so and I'll hold the arc here.)
