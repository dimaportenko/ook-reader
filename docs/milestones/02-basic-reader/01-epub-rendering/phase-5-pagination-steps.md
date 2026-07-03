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
   `Nav::apply`. *(planned — Step 3)*

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

## Step 3 — review & refactor: bundle reader state into a `Copy` `ReaderState`

Steps 1–2 grew `Reader` to **five signals** (`chapter`, `page`, `page_count`,
`pending_fragment`, `pending_last`) plus a `chapter_count`, and the transitions that touch
them are scattered across two `NavRow`s and three bridge arms. Step 2's `Nav::apply` killed
the worst duplication, but the applier still has to be *handed* three signals at every call
site, and "what may mutate the reader" has no single home. This step gives it one.

The move rests on one Dioxus fact: **`Signal<T>` is `Copy`**, so a *struct of signals* is
`Copy` too. Bundle them and the bundle threads into the future closure, both `NavRow`s, and
the render body with no `clone()` — while each field stays an independent signal, so
fine-grained reactivity is untouched.

**Runnable check.** This is a *behavior-preserving* refactor, so the check is that nothing
observable moved:

- `cargo test` — the Step-2 deciders `on_next`/`on_prev` **stay free functions** (only the
  appliers get bundled), so `page_nav_rolls_over_chapter_boundaries` and
  `paging_clamps_at_both_ends` pass **unchanged**. If a test needs editing, the refactor
  changed behavior — stop and reconsider.
- `cargo clippy` clean.
- Eyeball under `dx serve`: Prev/Next on both rows behave exactly as before — page clamps at
  the column count, rolls over chapter edges, prev-over-a-boundary lands on the last page,
  links and scroll still update the labels. The `ReaderState` *methods* touch signals, so
  they can't be unit-tested without a runtime — the eyeball is their check.

**Minimal implementation.** A `Copy` struct of signals, built by a custom hook, with one
method per transition.

```rust
#[derive(Clone, Copy)]
struct ReaderState {
    chapter: Signal<usize>,
    page: Signal<usize>,
    page_count: Signal<usize>,
    pending_fragment: Signal<Option<String>>,
    pending_last: Signal<bool>,
    chapter_count: usize, // constant for the book's life — a plain value, not a signal
}

// A custom hook — the `use_` prefix flags that it calls `use_signal`, so it must be
// invoked once, unconditionally, at the top of the component (hook rules).
fn use_reader_state(chapter_count: usize) -> ReaderState {
    ReaderState {
        chapter: use_signal(|| 0),
        page: use_signal(|| 0),
        page_count: use_signal(|| 0),
        pending_fragment: use_signal(|| None),
        pending_last: use_signal(|| false),
        chapter_count,
    }
}
```

Every transition becomes a method. `mut self` is a *local mutable copy* (the struct is
`Copy`); its `Signal` fields still point at the same runtime storage, so `.set()` through the
copy mutates the real state.

```rust
impl ReaderState {
    // page nav — the two deciders feed one applier
    fn page_prev(mut self) {
        self.apply(on_prev(self.page(), self.chapter()));
    }
    fn page_next(mut self) {
        self.apply(on_next(self.page(), self.page_count(), self.chapter(), self.chapter_count));
    }
    fn apply(mut self, nav: Nav) {
        match nav {
            Nav::Stay => {}
            Nav::Page(p) => self.page.set(p),
            Nav::Chapter { index, seek: Seek::First } => {
                self.page.set(0);
                self.chapter.set(index);
            }
            Nav::Chapter { index, seek: Seek::Last } => {
                self.chapter.set(index);
                self.pending_last.set(true);
            }
        }
    }

    // chapter nav (top row)
    fn chapter_prev(mut self) {
        self.page.set(0);
        self.chapter.set(prev_index(self.chapter()));
    }
    fn chapter_next(mut self) {
        self.page.set(0);
        self.chapter.set(next_index(self.chapter(), self.chapter_count));
    }

    // bridge messages
    fn follow_link(mut self, target: epub::LinkTarget) {
        self.chapter.set(target.spine_index);
        self.page.set(0);
        self.pending_fragment.set(target.fragment);
    }
    fn on_scroll(mut self, p: usize) {
        self.page.set(p);
        self.pending_fragment.set(None);
    }
    fn on_pages(mut self, pages: usize) {
        self.page_count.set(pages);
        if self.pending_last() {
            self.page.set(pages.saturating_sub(1));
            self.pending_last.set(false);
        }
    }
}
```

The component collapses to wiring — `state` sprinkled into the future closure, both rows, and
the labels with no `clone()`:

```rust
let state = use_reader_state(docs.len());
// … in the while-let: state.follow_link(target) / state.on_scroll(p) / state.on_pages(pages)
NavRow { on_prev: move |_| state.chapter_prev(), on_next: move |_| state.chapter_next(), label: chapter_label }
NavRow { on_prev: move |_| state.page_prev(),    on_next: move |_| state.page_next(),    label: page_label }
```

Pull the labels into `format!` locals — `"Chapter {state.chapter() + 1} of …"` with
field-access inside the format braces is the kind of expression the rsx string parser is iffy
about, and a local sidesteps the question.

**Why it works.** A `Signal<T>` is a handle into the runtime's slot map; copying the handle
doesn't copy the value, so a `#[derive(Copy)]` struct of signals shares one backing store
across every copy. That's why `mut self` mutating a *copy* still moves the real state, and why
`state` drops into the async task and both event handlers without the clone/move juggling five
bare signals would force at each site. Bundling doesn't coarsen reactivity the way a single
`Signal<ReaderData>` would: each field is still its own signal, so writing `page` re-runs only
what reads `page`. The deciders stay pure free functions — the tests don't move — and every
*mutation* now has exactly one name and one home.

**Scope note.**
- **Bigger blast radius than the duplication that prompted it.** This touches both `NavRow`s
  and all three bridge arms, not just the page-nav closures. If you only wanted the dedup,
  Step 2's `Nav::apply` already delivered it; `ReaderState` is the move when you want *one
  home* for all reader state — it tends to pay off right as the next piece of state arrives
  (a bookmark, a scroll-restore target).
- **Hooks hidden in a constructor.** `use_reader_state` must stay unconditional and
  called-once, like any hook; the `use_` name is the reminder.
- **The alternative I'd not pick here:** a single `Signal<ReaderData>` over a plain struct —
  simpler type, but coarser reactivity (any field write re-runs everything reading it) and
  `.write()` borrow-juggling. Struct-of-signals fits this reader better.
- **`chapter_count` stays a plain `usize`**, re-passed each render. It's constant per book, so
  there's nothing to make reactive.
