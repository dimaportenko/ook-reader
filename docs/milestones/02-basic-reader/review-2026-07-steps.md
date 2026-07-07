# Codebase review (July 2026) — refactor backlog & build log

[← Milestone 2](README.md) · **Status:** 🚧 in progress (R1, R2 done) ·
relates to: [Phase 6 — Library & Import](02-library/phase-6-library.md)

Findings from a full-codebase review (all 15 tests green, clippy clean at the time).
Each finding is written as a normal learn-by-building step — runnable check first, minimal
implementation sketch, why it works — to be picked up **independently and in any order**,
except where the "when to do it" note says otherwise. These are refactors and hardening,
not features: they ride *alongside* Phase 6, whose next feature step (Step 2, the
`rusqlite` store) stays the main line of work.

## The crux

The reader works, and the pure-logic core (`BridgeMsg::parse`, `on_next`/`on_prev`,
`resolve_internal_link`) is well separated and well tested. What the review found is the
gap between *working* and *idiomatic*: data that should be shared is being cloned
(R1), functions own responsibilities that belong to their callers (R2), errors are stringly
boxes about to meet user-supplied files (R3), two code paths mutate the same state (R4), a
one-variable CSS change costs a full document reload (R5), and a handful of small hygiene
holes (R6). Most of these are cheapest to fix **before** Phase 6 builds on top of them.

## Step plan

*(independent unless noted; suggested order R1 → R2 → R4 → R6 → R3 → R5)*

- [x] **R1 — Stop cloning the whole book every render.** Wrap the spine in `Rc` so
      `use_hook` clones a pointer, not every chapter's XHTML. *(tiny; do first — R2 touches
      the same line)*
- [x] **R2 — Open the EPUB once: pass `&Epub`, not paths.** `load_spine`/`read_metadata`
      become transformations of an already-open book. *(land before Phase 6 Step 5, which
      un-hardcodes `BOOK`)*
- [ ] **R3 — A real error type (`thiserror`).** Replace `Box<dyn Error>` + `expect` on the
      open path. *(pairs naturally with Phase 6 Step 6's "tidy error handling")*
- [ ] **R4 — Route chapter buttons through the `Nav` state machine.** One `apply` path for
      all navigation; fixes a real quirk on the last chapter.
- [ ] **R5 — Turn pages without reloading the iframe.** postMessage the new page into the
      injected listener instead of regenerating `iframe.src`. *(biggest step; defer until
      after Phase 6 lands — split with `lbb:refine` when picked up)*
- [ ] **R6 — Hygiene batch.** Fragment sanitization, case-insensitive content types, the
      "Page 1 of 0" label. *(three small test-first fixes in one sitting)*

---

## R1 — Stop cloning the whole book every render

`Reader` stores the spine as a bare `Vec<SpineDoc>` in `use_hook` (`main.rs:107`). In
Dioxus 0.7, `use_hook` stores the value once but returns a **clone of it on every render**
— so every page turn and every bridge message deep-copies the full XHTML of all 15
chapters. `App` already has the right pattern for its `Epub`: wrap it in `Rc` and clone the
*handle*.

**Runnable check.** This is a behavior-preserving refactor, so the existing suite is the
spec: `cargo test` green and `cargo clippy` clean before and after, plus a `dx serve`
eyeball that paging, chapter nav, and TOC links still work. No new test — there's no
observable behavior to pin down, only an allocation profile.

**Minimal implementation.** Two signature-level changes in `main.rs`:

```rust
// Reader — wrap the loaded spine in Rc at creation:
let docs = use_hook(|| Rc::new(epub::load_spine(BOOK).expect("bundled epub should load")));

// use_bridge — accept the shared handle instead of an owned Vec:
fn use_bridge(state: ReaderState, docs: Rc<Vec<epub::SpineDoc>>) { /* body unchanged */ }
```

Everything else compiles as-is: `docs.len()`, `&docs[chapter()]`, and
`resolve_internal_link(&docs, …)` all keep working because `Rc<Vec<T>>` derefs to
`Vec<T>` (and on through to `&[T]`) automatically.

**Why it works.** `Rc<T>` is a reference-counted pointer: `clone()` bumps an integer and
copies eight bytes, regardless of how big the book is. The clone `use_hook` hands back each
render — and the one inside `use_bridge`'s restartable closure — becomes a pointer copy
instead of a megabyte-scale deep copy. The general rule this teaches: when a `.clone()`
exists only to satisfy ownership (not because you need an independent copy to mutate),
share a handle instead of duplicating the payload.

**Scope note.** Don't reach for `Arc` — the Dioxus desktop UI is single-threaded, and `Rc`
is the honest (and cheaper) choice until something actually crosses a thread. R2 rewrites
the same `use_hook` line, which is why R1 goes first.

> **Status:** done — committed in `ffe692a` (15 tests green). Landed together with R2,
> which rewrites the same `Reader` `use_hook` line, so the two were inseparable in the diff.

---

## R2 — Open the EPUB once: pass `&Epub`, not paths

The book is currently opened **three times**: `App` opens it for the asset handler
(`main.rs:62`), `load_spine(BOOK)` opens it again (`epub.rs:44`), and `read_metadata(path)`
a third time (`epub.rs:223`). Each open re-reads the zip central directory and re-parses
the OPF. The root cause is the signatures: `load_spine` and `read_metadata` each *own* the
"open the file" step instead of receiving an open book.

**Runnable check.** Again a refactor, so the suite is the safety net — but the tests
themselves change shape, and that new shape is the point:

```rust
#[test]
fn loads_spine_in_reading_order() {
    let epub = Epub::open(crate::BOOK).expect("should open the bundled epub");
    let docs = load_spine(&epub).expect("bundled epub should load");
    // …existing assertions unchanged…
}
```

Update `loads_spine_in_reading_order`, `ignores_external_links`,
`resolves_contents_link_to_doc_and_fragment`, and `reads_title_and_author_from_metadata`
this way; all 15 tests stay green.

**Minimal implementation.**

1. Change the two signatures in `epub.rs` — the bodies *shrink* (the `Epub::open(path)?`
   line moves out):

   ```rust
   pub(crate) fn load_spine(epub: &Epub) -> Result<Vec<SpineDoc>, Box<dyn std::error::Error>>
   pub(crate) fn read_metadata(epub: &Epub) -> Result<BookMeta, Box<dyn std::error::Error>>
   ```

2. In `App`, the existing `Rc<Epub>` becomes the single source: share it with `Reader` via
   context (`use_context_provider(|| epub.clone())` in `App`,
   `use_context::<Rc<Epub>>()` in `Reader`), and build the spine from it:

   ```rust
   let docs = use_hook(|| Rc::new(epub::load_spine(&epub).expect("bundled epub should load")));
   ```

**Why it works.** Functions that take `&Epub` are pure transformations of an already-open
book — where the bytes came from is the *caller's* concern. That inversion is the classic
Rust "take the borrowed, general form" rule (`&str` over `String`, `&Epub` over `path`),
and it pays twice here: the zip is parsed once per book instead of three times, and the
functions become trivially testable against any `Epub` however it was opened. Context
(rather than a prop) fits because the `Epub` is app-wide ambient state, the same reason the
asset handler already hangs off `App`.

**When to do it / Phase 6 interplay.** Land this **before Phase 6 Step 5** ("open a book →
reader renders it") — Step 5 is exactly where `const BOOK` comes out, and it's much easier
when the open already happens in one place. Note it also adjusts Phase 6 Step 3's sketch:
the import flow becomes *picker path → `Epub::open(path)?` → `read_metadata(&epub)`* —
open once at the boundary, borrow inward.

> **Status:** done — committed in `ffe692a` (15 tests green). Signatures take `&Epub` (the
> general borrowed form), and `App` shares its `Rc<Epub>` with `Reader` via context.

---

## R3 — A real error type with `thiserror`

Today fallible EPUB operations return `Box<dyn std::error::Error>`, and the call sites
panic (`expect`) — acceptable while the only book is bundled and either loads or the app is
useless anyway. Phase 6 changes the contract: **import means user-supplied files** that
will be corrupt, DRM'd, renamed, or not EPUBs at all, and the UI needs to say *what went
wrong*, not crash.

**Runnable check.** A `#[test]` that a bad path produces a *matchable* error, not a panic
and not an opaque box:

```rust
#[test]
fn opening_a_missing_file_reports_an_open_error() {
    let err = open_book("/no/such/book.epub").unwrap_err();
    assert!(matches!(err, BookError::Open(_)), "got {err:?}");
    // The Display text should carry rbook's underlying cause for the UI/logs.
    assert!(!err.to_string().is_empty());
}
```

**Minimal implementation.** One enum in `epub.rs`, wrapping the underlying `rbook` error
via `#[from]` (check the exact error-type path on [docs.rs/rbook](https://docs.rs/rbook)
for the pinned 0.7 version — it's whatever type today's `?` conversions into
`Box<dyn Error>` are coming from):

```toml
# Cargo.toml
thiserror = "2"
```

```rust
#[derive(Debug, thiserror::Error)]
pub(crate) enum BookError {
    #[error("failed to open EPUB: {0}")]
    Open(#[from] /* rbook's error type */),
}
```

Then change `load_spine` and `read_metadata` to return `Result<_, BookError>`; the `?`s in
their bodies keep compiling because `#[from]` generates the conversion `?` needs. Grow the
enum only when a *distinct* failure needs distinct handling (e.g. a `MissingFile` variant
in Phase 6 Step 5 for the dangling-path case the phase doc already calls out).

**Why it works.** `thiserror` derives the two traits an error type needs — `Display` (the
`#[error("…")]` string) and `std::error::Error` (with `source()` wired to the wrapped
cause) — plus the `From` impl that lets `?` convert automatically. Compared to
`Box<dyn Error>`, the enum gives callers something to `match` on, which is exactly what a
UI needs to choose between "that file isn't an EPUB" and "that book has moved". Writing one
by hand (rather than reaching for `anyhow`) is deliberate here: it teaches `From`,
`Display`, and `?`-conversion — the machinery `anyhow` hides.

**When to do it.** Pairs naturally with **Phase 6 Step 6** ("tidy error handling") — but
doing it earlier makes Steps 3 and 5 cleaner, since import and open are the two places
these errors surface to the user. The `expect`s in `App`/`Reader` can stay until Step 5
gives the UI somewhere to show a failure.

> **Status:** pending.

---

## R4 — Route chapter buttons through the `Nav` state machine

Page navigation goes through a clean pure pipeline — compute a `Nav`, then `apply` it —
testable without Dioxus. But `chapter_prev`/`chapter_next` (`nav.rs:77-87`) mutate `page`
and `chapter` directly: a second code path touching the same state, with its own helpers
(`prev_index`/`next_index`) duplicating the boundary logic. It also hides a real quirk:
**"Next chapter" on the last chapter currently resets you to page 0** (the clamp returns
the same index, but `page.set(0)` runs unconditionally) instead of doing nothing.

**Runnable check.** Two new pure-function tests in `nav.rs`, mirroring the existing
`on_next`/`on_prev` style — note the last-chapter case *locks in the fix*:

```rust
#[test]
fn chapter_nav_steps_and_clamps() {
    assert_eq!(on_chapter_next(0, 15), Nav::Chapter { index: 1, seek: Seek::First });
    assert_eq!(on_chapter_prev(5),     Nav::Chapter { index: 4, seek: Seek::First });
    // At the edges nothing happens — including no page reset on the last chapter.
    assert_eq!(on_chapter_next(14, 15), Nav::Stay);
    assert_eq!(on_chapter_prev(0),      Nav::Stay);
}
```

**Minimal implementation.** Two pure functions beside `on_next`/`on_prev`, then the
methods become one-liners through `apply`; `prev_index`/`next_index` and their test are
deleted (the new tests cover the same boundaries):

```rust
fn on_chapter_next(chapter: usize, chapter_count: usize) -> Nav {
    if chapter + 1 < chapter_count {
        Nav::Chapter { index: chapter + 1, seek: Seek::First }
    } else {
        Nav::Stay
    }
}
// on_chapter_prev(chapter): same shape, guarded by chapter > 0, index: chapter - 1.

pub(crate) fn chapter_next(self) {
    let chapter = self.data.chapter();
    self.apply(on_chapter_next(chapter(), self.chapter_count));
}
```

**Why it works.** With every mutation funneled through `apply`, the `Nav` enum becomes the
*complete* vocabulary of "things that can happen to reading position" — there is exactly
one place that writes state, and everything upstream is pure and testable. It also makes a
product decision visible in the types: the chapter *buttons* seek `First` while paging
*backwards across a boundary* seeks `Last`. Before, that asymmetry was an accident of two
code paths; now it's a deliberate choice spelled `Seek::First` vs `Seek::Last` at two call
sites — and changeable in one line if you ever decide otherwise.

**Scope note.** Behavior change is limited to the last-chapter quirk (now `Stay` instead of
a surprise page reset) — everything else is identical, which the untouched existing tests
confirm.

> **Status:** pending.

---

## R5 — Turn pages without reloading the iframe

Every page turn re-runs `render_document_url`: re-inject CSS with a new `--ook-page`
value, re-base64 the whole chapter, swap `iframe.src` — a full document reload (reparse,
scripts rerun, pages-probe refires) for what is conceptually *changing one CSS variable*.
It works, but costs flicker on image-heavy books and makes page turns feel heavier than
they are. The message bridge already runs iframe → parent; this step adds the reverse
direction.

**Runnable check.** Two layers:

- Unit tests in `epub.rs`, same shape as `injects_page_count_probe_before_head_close`: a
  new `inject_page_listener` embeds its message kind (`ook-set-page`) and a
  `setProperty("--ook-page"` call, before `</head>`, leaving the body intact. And
  `render_document_url`'s output no longer varies with the page — same doc, same URL.
- `dx serve` eyeball: Next/Prev page is instant with no white flash; chapter changes still
  reload; TOC fragment links still land on the right page.

**Minimal implementation sketch.** (This is the biggest step in the backlog — when you pick
it up, run `lbb:refine` to split it; roughly 5a = the injected listener + tests, 5b = the
Rust side.)

1. **Inside the chapter** — a new injection, alongside the existing ones:

   ```javascript
   window.addEventListener('message', function(e) {
       if (!e.data || e.data.kind !== 'ook-set-page') return;
       document.documentElement.style.setProperty('--ook-page', e.data.page);
   });
   ```

2. **On the Rust side** — `iframe_src` must stop depending on `page()` (that dependency is
   what triggers the reload), so `render_document_url` renders at page 0 and the current
   page is pushed imperatively: a `use_effect` in `Reader` that reads `page()` and evals

   ```javascript
   document.querySelector('iframe').contentWindow.postMessage({ kind: 'ook-set-page', page: N }, '*');
   ```

3. The `pending_last` flow ("land on the last page when paging backwards across a
   chapter") keeps working unchanged: `on_pages` sets `page`, and the effect pushes it.

**Why it works.** Dioxus reloads the iframe because the `src` attribute is a reactive
value — remove `page` from the inputs of `iframe_src` and Dioxus has nothing to re-render.
The CSS variable was *already* the pagination mechanism (`transform:
translateX(calc(var(--ook-page) * -100vw))`); this step just changes who sets it — a
postMessage instead of a rebuilt document. `use_effect` is the right hook because pushing
a message into the DOM is a side effect that must run *after* render, re-running exactly
when the signals it reads (`page`) change.

**When to do it.** After Phase 6 lands. It rewires the reader's render path, and doing that
mid-phase risks destabilizing the thing Steps 4–5 need to wire the library into. Longer
term (recorded here, not part of this step): serving chapters through the existing
`use_asset_handler` route instead of base64 data URLs would shrink memory and make
relative links/images resolve for free.

> **Status:** pending.

---

## R6 — Hygiene batch: fragment sanitization, content types, page label

Three small, independent fixes — each a test + a few lines — batched as one sitting.

**(a) Validate fragments before injecting them into JavaScript.** `inject_fragment_scroll`
(`epub.rs:138`) interpolates the link fragment straight into
`getElementById("{fragment}")` — a crafted href ending `#x");…` breaks out of the string.
Not a real privilege escalation (chapter content already runs its own scripts in the
sandbox), but injection hygiene should be habit.

```rust
#[test]
fn fragment_scroll_rejects_unsafe_fragments() {
    let xhtml = r#"<html><head><t/></head><body/></html>"#;
    // A fragment that tries to break out of the JS string is not injected at all.
    let out = inject_fragment_scroll(xhtml, r#"x");alert(1);("#);
    assert_eq!(out, xhtml);
    // A normal id still is.
    assert!(inject_fragment_scroll(xhtml, "chap-1.2").contains("getElementById"));
}
```

Sketch: a `fn is_safe_fragment(&str) -> bool` allowing ASCII alphanumerics plus `_ - . :`
(the characters real-world ids use), checked at the top of `inject_fragment_scroll` —
return the input unchanged when it fails. *Why:* a whitelist over an escape function
because it's impossible to get wrong — you can't forget an escape case you never allow in.

**(b) Case-insensitive content types.** `content_type_for` (`epub.rs:23`) matches the
extension verbatim, so a `COVER.JPG` inside a zip comes back `application/octet-stream`.

```rust
#[test]
fn content_type_ignores_extension_case() {
    assert_eq!(content_type_for("OEBPS/COVER.JPG"), "image/jpeg");
}
```

Sketch: lowercase the extension (`to_ascii_lowercase()`) before the `match` — which means
matching on `ext.as_str()`, a small but instructive `String` vs `&str` moment.

**(c) No more "Page 1 of 0".** Before the pages-probe reports, `page_count` is 0 and the
nav row renders a nonsense label. Pure display fix in `Reader`: when `page_count() == 0`,
show a placeholder (`"Page …"`) instead of the formatted count. Verify by `dx serve`
eyeball — the placeholder flashes briefly on chapter load, then the real count appears.

**Recorded but deferred** (not steps, just so they're not forgotten):

- `BRIDGE_JS` accepts `message` events from any origin — fine in a desktop webview where
  only our iframe posts, but wants a stricter check before any web build.
- `epub.rs` unconditionally imports `dioxus::desktop`, so
  `--no-default-features --features web` does not compile today. The fix is
  `#[cfg(feature = "desktop")]` gating around the asset handler — a Milestone 4 concern.
- The `block v0.1.6` future-incompat warning from cargo is a transitive dependency of the
  webview stack; nothing actionable until wry bumps it upstream.

> **Status:** pending.
