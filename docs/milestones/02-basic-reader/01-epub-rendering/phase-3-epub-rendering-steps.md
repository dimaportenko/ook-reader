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

1. **Load the spine into `Vec<String>`** — pure Rust, `cargo test` against the real book.
   *(rbook, `Result`/`?`, `Vec`, ownership)*
2. **Render the current document** — show `docs[current]` via `dangerous_inner_html` in a
   scrollable view (`current` fixed at 0). Eyeball under `dx serve`. *(Dioxus element +
   `dangerous_inner_html`)*
3. **Turn pages** — a `use_signal` index; Next/Prev mutate it, clamped to `0..docs.len()`.
   Eyeball: page through all 15 items. *(signals, event handlers, clamping)*

---

## Step 1 — load the spine into a `Vec<String>`

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
