# Phase 5 — Pagination — build log

[← Phase doc](phase-5-pagination.md)

Per-step test → minimal code → why, appended newest-last. The
[phase doc](phase-5-pagination.md)'s "Planned steps" checklist is the high-level index;
this file is the detail and the build log.

## The crux

Page count is a **layout-derived** quantity — Rust can't compute it. How many columns a
chapter breaks into depends on font metrics, viewport width, and the book's own CSS, all of
which live in the webview. So the shape is: **measure in JS after the page lays out → report
the number back over the bridge you already built → store it in a signal → use it to clamp
and label.** Rust owns the decisions (clamp, label, later roll-over); the webview owns the
measurement.

Every seam this needs already exists: the `inject_*` injection pattern in `epub.rs`, the
`postMessage` → `dioxus.send` bridge, and the `ook-scroll` arm that already sets `page` from
a JS-reported number. Page-count is that same move with a new message kind.

## Step plan

1. **Measure & report the page count** from the iframe on `load`; store in a `page_count`
   signal; show `Page X / N`. *(done — Step 1)*
2. **Clamp Page-Next and roll turns over chapter boundaries** — one pure `Nav` decider
   (`on_next`/`on_prev`) that both clamps to the count and spills across chapter edges, with
   a `#[test]`. Merges the originally-separate "clamp" and *(optional)* "roll-over" steps,
   since they were built together. *(done — Step 2)*
3. **Review & refactor** the pagination phase — consolidate the reader's five signals into a
   single `Copy` `ReaderState` that owns every state transition (`page_next`, `chapter_prev`,
   `follow_link`, `on_pages`, …), retiring the three-signal threading left by Step 2's
   `Nav::apply`. *(done — Step 3)*

---

## Step 1 — measure page count in the iframe, report it, display it

**Runnable check (two parts).**

*Part A — a pure `#[test]` for the injection seam* (mirrors `injects_fragment_scroll_before_head_close`).
Proves the probe is wired into the document; needs no browser:

```rust
#[test]
fn injects_page_count_probe_before_head_close() {
    let xhtml = r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body><p>Hi</p></body></html>"#;

    let out = inject_page_count_probe(xhtml);

    // reports back over the bridge under its own message kind …
    assert!(out.contains("ook-pages"));
    // … derives the count from the laid-out width vs the viewport …
    assert!(out.contains("scrollWidth"));
    assert!(out.contains("innerWidth"));
    // … is injected into the head so it parses before the body it measures …
    assert!(out.find("ook-pages").unwrap() < out.find("</head>").unwrap());
    // … and leaves the original document intact.
    assert!(out.contains("<p>Hi</p>"));
}
```

*Part B — eyeball under `dx serve` + `cargo clippy`:* the second NavRow reads `Page 1 / N`
where **N is plausible for the chapter** (a long story shows more pages than the short
cover) and it **updates when you switch chapters**. This is what confirms the *number is
right* — the unit test only confirms the seam.

**Minimal implementation.**

New function in `src/epub.rs`, alongside `inject_fragment_scroll`:

```rust
pub(crate) fn inject_page_count_probe(xhtml: &str) -> String {
    let script = r#"<script type="text/javascript">
    //<![CDATA[
        window.addEventListener('load', function() {
            // Each page advances by one viewport (translateX steps of 100vw),
            // and the column pitch is exactly 100vw, so total pages =
            // laid-out content width / viewport width.
            var count = Math.max(
                1,
                Math.ceil(document.body.scrollWidth / window.innerWidth)
            );
            window.parent.postMessage({ kind: 'ook-pages', count: count }, '*');
        });
    //]]>
    </script>"#;

    insert_before_head_close(xhtml, script)
}
```

Wire it into `render_document_url` (runs every render, like the link bridge):

```rust
pub(crate) fn render_document_url(doc: &SpineDoc, page: usize, fragment: Option<&str>) -> String {
    let paged = inject_pagination_css(&doc.xhtml, page);
    let bridged = inject_link_bridge(&paged);
    let probed = inject_page_count_probe(&bridged);          // <-- new
    let prepared = match fragment {
        Some(frag) => inject_fragment_scroll(&probed, frag),
        None => probed,
    };
    to_xhtml_data_url(&prepared)
}
```

In `src/main.rs`, a new signal (near `page`) and a new bridge arm:

```rust
let mut page_count = use_signal(|| 1usize);
```

```rust
// in the while-let, alongside the "scroll:" arm
} else if let Some(n) = msg.strip_prefix("pages:") {
    if let Ok(n) = n.parse::<usize>() {
        page_count.set(n.max(1));
    }
}
```

Extend the JS bridge dispatcher (the `document::eval` string) to forward the new kind:

```js
if (e.data.kind === 'ook-pages') {
    dioxus.send("pages:" + e.data.count);
}
```

And show it in the Page NavRow label:

```rust
label: "Page {page() + 1} / {page_count()}",
```

**Why it works.** The probe runs on `load`, i.e. *after* the browser has laid the body out
into columns, so `document.body.scrollWidth` is the full multi-column span, not the clamped
viewport — that's exactly why the measurement can't live in Rust and why it's on the `load`
event, not inline. Dividing by `window.innerWidth` gives the page count because `translateX`
steps the body by one viewport per page and the column pitch is
`column-width (100vw − 48px) + column-gap (48px) = 100vw`, so "content width ÷ viewport" *is*
the number of pages. The count rides the existing `postMessage`→`dioxus.send`→`recv` bridge
exactly like `ook-scroll` already does. `page_count` is a plain `use_signal`; writing it from
the async bridge task schedules a re-render, so the label updates the moment the count lands.

**Scope note.**
- **No clamping yet** — Page-Next can still walk past the last column. That's deliberately
  **Step 2**, where the count becomes a pure-Rust clamp with its own test. Step 1 only
  *surfaces* the number.
- **Re-measures on every page turn** — `--ook-page` is baked into the data URL, so each turn
  reloads the iframe and re-fires the probe. Same count reported → harmless, just redundant.
  Decoupling page-turn from full reload is a candidate for **Step 4**, not now.
- **`scrollWidth` is the thing to verify** — if N reads obviously wrong (always 1, or wildly
  high), that's the measurement to debug; the fallback is a trailing sentinel's `offsetLeft`.
  Flag it and we'll adjust via `lbb:refine`.

**Deviations from the sketch (recorded during the build).** Two small changes landed beyond
the Step-1 sketch above:

- **The probe also listens on `resize`, not just `load`.** `load` fires once; it never
  re-fires when the viewport width changes, which would leave a stale count after a window
  resize. The report function is now named and bound to *both* events:

  ```js
  const report = function() {
      var count = Math.max(1, Math.ceil(document.body.scrollWidth / window.innerWidth));
      window.parent.postMessage({ kind: 'ook-pages', count: count }, '*');
  };
  window.addEventListener('load', report);
  window.addEventListener('resize', report);
  ```

- **`page_count` starts at `0`, not `1`.** `0` reads as an explicit "not measured yet"
  sentinel, which Step 2's `on_next` guard (`page_count() > 0`) keys off to avoid rolling
  over before the chapter has actually reported its column count. The label shows
  `Page 1 of 0` for the brief window before the first `ook-pages` arrives.

> **Status:** done — committed in `5937727` alongside Step 2 (13 tests green).

---

## Step 2 — clamp Page-Next and roll turns over chapter boundaries

Step 1 only *surfaced* the count; Page-Next could still walk past the last column into blank
pages. This step makes the count *govern* paging, and while we're deciding "is there a next
page?" we get the chapter roll-over almost for free: the same branch that says "no next page
in this chapter" is exactly where "go to the next chapter" belongs.

**The asymmetry to notice.** Forward is synchronous — going to page 0 of the next chapter
needs no measurement. Backward is *not*: "the previous chapter's **last** page" needs that
chapter's page count, which doesn't exist until its iframe loads and the probe reports. So
prev-over-a-boundary can't finish in the click handler — it has to defer, the same way
`pending_fragment` already does. That deferral is a `pending_last` flag resolved in the
`pages:` arm.

**Runnable check.** A pure `#[test]` for the decision logic — no signals, no browser — added
to the `test` mod in `main.rs`, mirroring `paging_clamps_at_both_ends`:

```rust
#[test]
fn page_nav_rolls_over_chapter_boundaries() {
    // within a chapter: plain page steps
    assert_eq!(on_next(0, 3, 0, 15), Nav::Page(1));
    assert_eq!(on_prev(2, 3), Nav::Page(1));

    // last page of a 3-page chapter → next chapter, first page
    assert_eq!(on_next(2, 3, 0, 15), Nav::Chapter { index: 1, seek: Seek::First });
    // last page of the LAST chapter → stay put
    assert_eq!(on_next(2, 3, 14, 15), Nav::Stay);

    // page 0 with a previous chapter → prev chapter, last page (resolved async)
    assert_eq!(on_prev(0, 3), Nav::Chapter { index: 2, seek: Seek::Last });
    // page 0 of chapter 0 → stay put
    assert_eq!(on_prev(0, 0), Nav::Stay);

    // edge: Next before the probe reports (page_count == 0) spills forward, skipping the
    // unmeasured chapter. Documents the race; flip the assertion if the guard changes.
    assert_eq!(on_next(0, 0, 0, 15), Nav::Chapter { index: 1, seek: Seek::First });
}
```

**Minimal implementation.** Two pure deciders plus the value types they return, kept next to
`next_index`/`prev_index`:

```rust
#[derive(Debug, PartialEq)]
enum Seek { First, Last }

#[derive(Debug, PartialEq)]
enum Nav {
    Stay,
    Page(usize),
    Chapter { index: usize, seek: Seek },
}

fn on_next(page: usize, page_count: usize, current: usize, len: usize) -> Nav {
    if page_count > 0 && page + 1 < page_count {
        Nav::Page(page + 1)
    } else if current + 1 < len {
        Nav::Chapter { index: current + 1, seek: Seek::First }
    } else {
        Nav::Stay
    }
}

fn on_prev(page: usize, current: usize) -> Nav {
    if page > 0 {
        Nav::Page(page - 1)
    } else if current > 0 {
        Nav::Chapter { index: current - 1, seek: Seek::Last }
    } else {
        Nav::Stay
    }
}
```

The Page NavRow handlers shrink to *applying* the `Nav`:

| `Nav` | applier |
|---|---|
| `Stay` | nothing |
| `Page(p)` | `page.set(p)` |
| `Chapter { index, seek: First }` | `page.set(0); current.set(index)` |
| `Chapter { index, seek: Last }` | `current.set(index); pending_last.set(true)` |

Concretely, the Page `NavRow` closures call the decider and `match` on the returned `Nav`.
(The prop key `on_next:` and the free function `on_next` don't collide — struct-field names
and function names live in separate namespaces, so `on_next(...)` inside the closure always
resolves to the free function.)

```rust
NavRow {
    on_prev: move |_| match on_prev(page(), current()) {
        Nav::Stay => {}
        Nav::Page(p) => page.set(p),
        Nav::Chapter { index, seek: Seek::First } => {
            page.set(0);
            current.set(index);
        }
        Nav::Chapter { index, seek: Seek::Last } => {
            current.set(index);
            pending_last.set(true);
        }
    },
    on_next: move |_| match on_next(page(), page_count(), current(), len) {
        Nav::Stay => {}
        Nav::Page(p) => page.set(p),
        Nav::Chapter { index, seek: Seek::First } => {
            page.set(0);
            current.set(index);
        }
        Nav::Chapter { index, seek: Seek::Last } => {
            current.set(index);
            pending_last.set(true);
        }
    },
    label: "Page {page() + 1} of {page_count}",
}
```

Both closures share the same two `Chapter` arms — the only difference is which decider feeds
the `match`: `on_prev` takes `(page, current)`, `on_next` also needs `page_count` and `len`
to know where the clamps and the book's end are. Notice the `match` *is* the whole body now:
every arm is one of the appliers from the table, and `Nav::Stay => {}` makes "do nothing at a
hard end" explicit rather than an implicit fall-through.

`Seek::Last` leaves the *number* to the deferred arm. A new signal holds the intent:

```rust
let mut pending_last = use_signal(|| false);
```

and the `pages:` arm — which already stores the count — resolves it once the target chapter
reports:

```rust
} else if let Some(pages) = msg.strip_prefix("pages:") {
    if let Ok(pages) = pages.parse::<usize>() {
        page_count.set(pages);
        if pending_last() {
            page.set(pages.saturating_sub(1));   // land on the last page
            pending_last.set(false);
        }
    }
}
```

**Why it works.** `on_next`/`on_prev` are total functions over `(page, page_count, current,
len)` — every end condition (clamp at the last column, spill forward, spill back, stay at
the book's two ends) is a branch, so a `#[test]` pins them all without a webview. The `Nav`
enum is the seam: the deciders own *what should happen*, the closures own *mutating signals*,
and neither leaks into the other. Forward navigation resolves immediately; backward-over-a-
boundary defers through `pending_last` because the previous chapter's last-page index is a
layout-derived quantity — the very same reason page count itself had to come back over the
bridge in Step 1. The jump reuses the exact `postMessage → dioxus.send → recv` path already
carrying `ook-scroll` and `ook-pages`.

**Scope note.**
- **Prev-over-a-boundary flashes page 0 first.** The new chapter renders at page 0, then
  snaps to its last page once measured — same double-load behavior as fragment-scroll.
  Accepted for now; removing it means decoupling page-turn from full iframe reload (Step 3
  territory).
- **The `page_count == 0` race** (Next before the first measurement skips the unmeasured
  chapter) is documented by the last test case rather than guarded away. Narrow — the probe
  fires on `load` — and cheap to change later if it bites.

**Deviations from the sketch (recorded during the build).**

- **`current`/`len` were renamed to `chapter`/`chapter_count`** in `main.rs` — the reader now
  reads `chapter()` / `chapter_count` everywhere. The decider parameter names in the sketch
  above still read `current`/`len`; the behavior is identical.
- **The duplicated `match` was extracted into a `Nav::apply` method.** Rather than repeat the
  four arms in both closures (the two code blocks above), the applier moved onto the enum,
  taking the signals it mutates — which are `Copy`, so they pass by value:

  ```rust
  impl Nav {
      fn apply(self, mut page: Signal<usize>, mut chapter: Signal<usize>, mut pending_last: Signal<bool>) {
          match self { /* the four arms, exactly once */ }
      }
  }
  ```

  so each closure shrinks to `on_prev(page(), chapter()).apply(page, chapter, pending_last)`.
  Step 3 takes this the rest of the way, folding all five signals into one `ReaderState` so
  the applier no longer has to be *handed* its signals.

> **Status:** done — committed in `5937727` (13 tests green, incl.
> `page_nav_rolls_over_chapter_boundaries`).

---

## Step 3 — review & refactor: bundle reader state into a `Store`

Steps 1–2 grew `Reader` to **five signals** (`chapter`, `page`, `page_count`,
`pending_fragment`, `pending_last`) plus a `chapter_count`, and the transitions that touch
them are scattered across two `NavRow`s and three bridge arms. Step 2's `Nav::apply` killed
the worst duplication, but the applier still has to be *handed* three signals at every call
site, and "what may mutate the reader" has no single home. This step gives it one.

The idiomatic 0.7 tool for exactly this is a **`Store`**. The 0.7 release added
`#[derive(Store)]` for the case its notes call out — *"signals are great for atomic state…
but they are difficult to use with nested state."* You write **plain data**, derive `Store`,
and the macro generates one accessor *method* per field that returns a scoped, `Copy`,
signal-like handle. Fields lazily become their own signals, so fine-grained reactivity is
preserved — writing `page` re-runs only what read `page` — without you hand-rolling five
`use_signal`s. And because the accessors are real methods (not raw `Signal` fields), reading
them never hits the `(self.page)()` field-call snag that a struct-of-signals forces.

**Runnable check.** This is a *behavior-preserving* refactor, so the check is that nothing
observable moved:

- `cargo test` — the Step-2 deciders `on_next`/`on_prev` **stay free functions** (only the
  appliers get bundled), so `page_nav_rolls_over_chapter_boundaries` and
  `paging_clamps_at_both_ends` pass **unchanged**. If a test needs editing, the refactor
  changed behavior — stop and reconsider.
- `cargo clippy` clean. (`Store` and `use_store` are in `dioxus::prelude` — no extra Cargo
  feature to enable.)
- Eyeball under `dx serve`: Prev/Next on both rows behave exactly as before — page clamps at
  the column count, rolls over chapter edges, prev-over-a-boundary lands on the last page,
  links and scroll still update the labels. The `ReaderState` *methods* touch the store, so
  they can't be unit-tested without a runtime — the eyeball is their check.

**Minimal implementation.** A plain data struct that derives `Store`, wrapped in a small
`Copy` `ReaderState` that also carries the constant `chapter_count`.

```rust
// The state as plain data — no signals in the source. `Default` gives the initial
// values (0 / None / false), which is what `use_store` seeds the store with.
#[derive(Store, Default)]
struct ReaderData {
    chapter: usize,
    page: usize,
    page_count: usize,
    pending_fragment: Option<String>,
    pending_last: bool,
}

// Wrap the store in our OWN type so the transitions can be plain inherent methods.
// (`Store<ReaderData>` is foreign, so methods on it would need an extension trait;
// `ReaderState` is ours.) Both `Store` and `usize` are `Copy`, so the wrapper is too.
#[derive(Clone, Copy)]
struct ReaderState {
    data: Store<ReaderData>,
    chapter_count: usize, // constant for the book's life — kept out of the store, not reactive
}

// A custom hook — the `use_` prefix flags the `use_store` call inside, so it must be
// invoked once, unconditionally, at the top of the component (hook rules).
fn use_reader_state(chapter_count: usize) -> ReaderState {
    ReaderState {
        data: use_store(ReaderData::default),
        chapter_count,
    }
}
```

Every transition becomes a method. `self` itself is `Copy` and needs no `mut` — writes go
through the store's accessors, not through `self`. Bind an accessor to a local and it behaves
like a plain signal: **call it to read, `.set()` to write**. One catch: `.set()` takes
`&mut self`, so a local you *write* through must be bound `mut` (`let mut chapter = …`) — exactly
the `let mut count = store.count()` idiom from the docs. A local you only *read* stays immutable,
and an unnamed temporary (`self.data.page().set(0)`) needs nothing at all.

```rust
impl ReaderState {
    // page nav — the two deciders (unchanged, still pure free fns) feed one applier
    fn page_prev(self) {
        let (page, chapter) = (self.data.page(), self.data.chapter());
        self.apply(on_prev(page(), chapter()));
    }
    fn page_next(self) {
        let (page, page_count, chapter) =
            (self.data.page(), self.data.page_count(), self.data.chapter());
        self.apply(on_next(page(), page_count(), chapter(), self.chapter_count));
    }
    fn apply(self, nav: Nav) {
        // bound `mut` because we write through them with `.set()` below
        let (mut page, mut chapter) = (self.data.page(), self.data.chapter());
        match nav {
            Nav::Stay => {}
            Nav::Page(p) => page.set(p),
            Nav::Chapter { index, seek: Seek::First } => {
                page.set(0);
                chapter.set(index);
            }
            Nav::Chapter { index, seek: Seek::Last } => {
                chapter.set(index);
                self.data.pending_last().set(true);
            }
        }
    }

    // chapter nav (top row)
    fn chapter_prev(self) {
        let mut chapter = self.data.chapter(); // `mut`: we both read and `.set()` it
        self.data.page().set(0);
        chapter.set(prev_index(chapter()));
    }
    fn chapter_next(self) {
        let mut chapter = self.data.chapter();
        self.data.page().set(0);
        chapter.set(next_index(chapter(), self.chapter_count));
    }

    // bridge messages
    fn follow_link(self, target: epub::LinkTarget) {
        self.data.chapter().set(target.spine_index);
        self.data.page().set(0);
        self.data.pending_fragment().set(target.fragment);
    }
    fn on_scroll(self, p: usize) {
        self.data.page().set(p);
        self.data.pending_fragment().set(None);
    }
    fn on_pages(self, pages: usize) {
        let (mut page, mut pending_last) = (self.data.page(), self.data.pending_last());
        self.data.page_count().set(pages);
        if pending_last() {
            page.set(pages.saturating_sub(1));
            pending_last.set(false);
        }
    }
}
```

The component collapses to wiring — `state` sprinkled into the future closure, both rows, and
the labels:

```rust
let state = use_reader_state(docs.len());
// … in the while-let: state.follow_link(target) / state.on_scroll(p) / state.on_pages(pages)
//     the untracked read for link resolution: let idx = *state.data.chapter().peek();
NavRow { on_prev: move |_| state.chapter_prev(), on_next: move |_| state.chapter_next(), label: chapter_label }
NavRow { on_prev: move |_| state.page_prev(),    on_next: move |_| state.page_next(),    label: page_label }
```

Pull the labels into `format!` locals, binding each accessor first so the read is a plain
positional arg (the rsx string parser is iffy about calls inside `"{…}"` braces):

```rust
let chapter = state.data.chapter();
let (page, page_count) = (state.data.page(), state.data.page_count());
let chapter_label = format!("Chapter {} of {}", chapter() + 1, state.chapter_count);
let page_label = format!("Page {} of {}", page() + 1, page_count());
```

Reading `chapter()` here in the render body subscribes the component to that field, so it
re-renders when the chapter changes — same behavior as the old bare `chapter()` signal read,
and the same `&docs[chapter()]` indexing still selects the current doc.

**Why it works.** A `Store<T>` is a `Copy` handle into the runtime; `#[derive(Store)]` gives it
one accessor method per field, and each accessor lazily materialises that field as its own
signal the first time it's touched. So `state` copies freely into the async task and both event
handlers (like any signal), while `page.set(…)` still re-runs only what read `page` — the
fine-grained reactivity a single `Signal<ReaderData>` would lose. Wrapping the store in our own
`ReaderState` buys two things: the transitions become plain `impl` methods (no extension trait
on the foreign `Store` type), and the constant `chapter_count` rides along without being made
reactive. The deciders stay pure free functions — the tests don't move — and every *mutation*
now has exactly one name and one home. Reading is `let page = self.data.page(); page()`:
`self.data.page()` is a genuine generated method, so calling its result reads the value with
none of the `(self.page)()` field-call ambiguity a raw `Signal` field forces.

**Scope note.**
- **Why `Store`, not a struct of signals.** Both bundle the state; the store wins on two
  counts here — the source is plain data (add a field = add a struct field, not another
  `use_signal` line), and its accessors are methods, so reads never need the `(self.page)()`
  parens dance. The struct-of-signals version is the fallback if you want zero new concepts.
- **`ReaderState` wraps the store on purpose.** Methods on `Store<ReaderData>` directly would
  need an extension trait (it's a foreign type); a thin wrapper of our own gets plain inherent
  `impl` and a home for `chapter_count`.
- **`self` is `Copy`, no `mut`.** Writes flow through the store's writable accessors, not
  through `self`, so the methods take `self` by value with nothing to mutate on the copy.
- **Reading: bind then call.** `let page = self.data.page(); page()` reads; `.peek()` reads
  untracked (used for the link-resolution `idx`); `.set()` writes. Binding the accessor to a
  local keeps each read/write to one clear line.
- **`mut` follows the write, not the type.** `.set()` borrows `&mut self`, so a local you
  write through needs `let mut …` even though the handle is `Copy`. Read-only locals stay
  immutable; unnamed temporaries (`self.data.page().set(0)`) need no binding. Reads use
  `Readable`/`&self`, writes use `Writable`/`&mut self` — that split is the whole rule.
- **`chapter_count` stays out of the store.** It's constant per book, so there's nothing to
  make reactive — a plain `usize` field on the wrapper, re-passed each render.
- **Bigger blast radius than the duplication that prompted it.** This touches both `NavRow`s
  and all three bridge arms. If you only wanted the dedup, Step 2's `Nav::apply` already
  delivered it; the store is the move when you want *one home* for all reader state — it pays
  off right as the next piece arrives (a bookmark, a scroll-restore target).

> **Status:** done — committed in `9787cf1` (13 tests green; deciders unchanged).

---

## Step 4 — model the bridge protocol as a `BridgeMsg` enum with a pure parser

After Step 3, every reader *mutation* has a named home on `ReaderState`, but the future's
`while let` still picks messages apart inline with `strip_prefix("link:")` /
`.parse::<usize>()` (`main.rs:191–210`). That's a stringly-typed protocol — the same shape
Step 2 already turned into pure, tested functions with `on_next`/`on_prev`. This step gives
the *incoming* half the same treatment: a `BridgeMsg` enum and one pure `parse` function,
leaving the loop body as pure dispatch onto the Step-3 methods.

The win is testability. The parser takes a `&str` and returns an `Option<BridgeMsg>` with no
signals, no runtime, no `.await` — so it's a plain unit test, exactly like the deciders. The
`ReaderState` methods stay the untestable-but-trivial glue; the *decoding* — the part with
branches and failure cases — moves behind a test.

**Runnable check.** A test on the parser, written first:

```rust
#[test]
fn bridge_parses_each_message_kind() {
    assert_eq!(BridgeMsg::parse("scroll:3"), Some(BridgeMsg::Scroll(3)));
    assert_eq!(BridgeMsg::parse("pages:12"), Some(BridgeMsg::Pages(12)));
    assert_eq!(
        BridgeMsg::parse("link:chapter2.xhtml#s3"),
        Some(BridgeMsg::Link("chapter2.xhtml#s3".to_string()))
    );
    // unknown prefixes and malformed numbers decode to None, never panic
    assert_eq!(BridgeMsg::parse("scroll:notanumber"), None);
    assert_eq!(BridgeMsg::parse("bogus:1"), None);
}
```

Then eyeball under `dx serve`: links, scroll, and page-count updates behave exactly as before
— this is behavior-preserving, so nothing observable moves.

**Minimal implementation.** An enum plus a `parse` that folds the current inline `strip_prefix`
ladder into one place:

```rust
#[derive(Debug, PartialEq)]
enum BridgeMsg {
    Link(String),
    Scroll(usize),
    Pages(usize),
}

impl BridgeMsg {
    fn parse(msg: &str) -> Option<BridgeMsg> {
        if let Some(href) = msg.strip_prefix("link:") {
            Some(BridgeMsg::Link(href.to_string()))
        } else if let Some(p) = msg.strip_prefix("scroll:") {
            p.parse().ok().map(BridgeMsg::Scroll)
        } else if let Some(pages) = msg.strip_prefix("pages:") {
            pages.parse().ok().map(BridgeMsg::Pages)
        } else {
            None
        }
    }
}
```

The loop body then collapses to dispatch:

```rust
while let Ok(msg) = bridge.recv::<String>().await {
    match BridgeMsg::parse(&msg) {
        Some(BridgeMsg::Link(href)) => {
            let idx = *state.data.chapter().peek();
            if let Some(target) = epub::resolve_internal_link(&docs, idx, &href) {
                state.follow_link(target);
            }
        }
        Some(BridgeMsg::Scroll(p)) => state.on_scroll(p),
        Some(BridgeMsg::Pages(pages)) => state.on_pages(pages),
        None => {} // unknown message — ignore, don't panic
    }
}
```

**Why it works.** The decode has three branches and two failure modes (unknown prefix,
unparseable number); pulling it into `parse` puts all of that behind a `&str -> Option`
signature that a test can hammer without a Dioxus runtime. The `link:` arm still needs
`resolve_internal_link` — that reads `docs`, so it stays in the loop, not the parser; the
parser's job ends at "which message, carrying what". The `None` arm makes an unknown message a
no-op instead of a silent `strip_prefix` fall-through, which is the same total behavior stated
explicitly. Note the protocol is still duplicated across the JS/Rust boundary (JS re-encodes
`kind: 'ook-link'` as `"link:" + raw`); this step owns the *Rust* half — the JS half is
Step 5's `const`.

**Scope note.**
- **Only the inbound half.** This models messages coming *from* the page. The outbound eval
  string is untouched here — see Step 5.
- **`resolve_internal_link` stays in the loop**, not the parser — it needs `docs`, and keeping
  the parser signal-free and arg-free is the whole point of the split.

> **Status:** done — committed in `6c619fd` (14 tests green, incl. the new
> `bridge_parses_each_message_kind`).

---

## Step 5 — relocate: nav cluster to a module, plus three glue tidies

Steps 3–4 leave `main.rs` doing five unrelated jobs: entry point, app shell, EPUB asset
serving, the nav domain, and the reader component. This step is pure *relocation* — no new
abstractions, no behavior change — to get `main.rs` back to "app wiring + components." It
bundles one real move and three small tidies; do them in any order, each is independently
revertable.

**Runnable check.** `cargo test` and `cargo clippy` stay green with **no test edits** — every
sub-move is behavior-preserving, so if a test needs touching, something changed; stop. Eyeball
under `dx serve` once at the end: paging, chapter nav, links, and scroll all behave as before.

**Minimal implementation.** Four moves:

1. **Nav cluster → its own module** (`reader/nav.rs`, or `nav.rs` if you keep it flat). Move
   `Seek`, `Nav`, `on_next`, `on_prev`, `prev_index`, `next_index`, `ReaderData`, `ReaderState`,
   `use_reader_state`, and the `#[cfg(test)]` block that tests them. `main.rs` gets a `mod nav;`
   and a `use nav::*;`. The deciders are the tested core; giving them a home is the point.
2. **Inline JS → a named `const`.** Lift the `document::eval` listener string (`main.rs:174–187`)
   to `const BRIDGE_JS: &str = r#"…"#;`, so the future reads `document::eval(BRIDGE_JS)`. This
   is the Rust-side of the "move javascript strings to js files" TODO.
3. **Bridge future → a `use_bridge(state, docs)` hook.** After Step 4 the loop body is clean
   dispatch, so wrapping the whole `use_future` in a custom hook keeps `Reader`'s body to
   render-wiring. *Order matters:* this only pays off once Step 4 has made the loop tidy —
   before that you'd just be relocating a mess.
4. **Asset handler → `epub`.** The ~25-line resource-serving closure in `App` (`main.rs:94–117`)
   is EPUB glue, not reader state. Move it behind `epub::register_asset_handler(epub)` so it
   lives next to the module it serves.

**Why it works.** None of these changes what runs — they change *where it lives*. A module
boundary (`nav`) turns "scroll up to find the deciders" into `use nav::*`; a `const` names the
JS blob so the future isn't interrupted by a wall of embedded script; a `use_bridge` hook makes
`Reader` read as "derive `iframe_src`, wire the bridge, render"; and moving the asset handler
puts EPUB-serving code in the EPUB module. The reason this is one step and not four is that
each is a few lines of cut-and-paste with the same runnable check — batching them avoids four
near-identical commits.

**Scope note.**
- **`use_bridge` is untestable glue** — its value is purely that `Reader` gets shorter. That's
  fine, but it's why it rides along with relocation rather than earning its own step the way
  Step 4's parser did.
- **`ReaderState` isn't pure-domain** — it holds a `Store`, so `nav.rs` depends on Dioxus. The
  *deciders* are the pure part; if you ever want a truly framework-free core, they're what to
  keep store-free (they already are — they take and return plain values).
- **Stop early if `main.rs` still reads fine.** This is a "the file got crowded" step; if it
  hasn't, defer it. Relocation with no behavior change is the easiest work to postpone.

> **Status:** done — move 1 (nav cluster → `src/nav.rs`) committed in `a1f3aaf`; moves 2–4
> (`BRIDGE_JS` const, `use_bridge` hook, `epub::use_register_asset_handler`) committed in
> `6e2861f`. 14 tests green throughout, no test edits — pure relocation. `main.rs` is now
> `main`/`App`/`NavRow`/`Reader` + the `BridgeMsg` protocol, with nav in `nav`, EPUB serving
> in `epub`, and the JS blob named. `use_register_asset_handler` and `use_bridge` carry the
> `use_` prefix since each calls a hook.
