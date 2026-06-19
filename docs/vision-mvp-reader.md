# Vision — MVP reader as a walking skeleton

How we build `ook-reader`: **thin vertical slices**, each one end-to-end and *actually
usable*, instead of building horizontal layers (a complete domain model, then a complete UI,
then persistence) before anything works. This is the [walking-skeleton][ws] approach.

[ws]: https://en.wikipedia.org/wiki/Walking_skeleton

The decision and its rationale are recorded in
[ADR-0001](adr/0001-walking-skeleton-vertical-slices.md).

## The principle

Every step must end with **something a real reader can do** that they couldn't before —
open a book, see its text, turn a page, resume where they left off. No step exists only to
"set up infrastructure." Infrastructure is pulled in by the slice that first needs it.

Two consequences that matter for *this* project:

- **The learning rides inside the slices.** The Rust + Dioxus fundamentals
  ([Phase 1](milestones/01-foundation/03-rust-dioxus-fundamentals/phase-1-learn-rust-dioxus.md))
  are not separate throwaway exercises — each concept is learned where a real feature needs
  it (signals when we turn pages, `serde` when we save position, traits when we abstract
  storage). A topic gets checked off when the slice that exercises it ships.
- **Hard domain decisions are deferred until a slice forces them.** We don't model the full
  EPUB spine/ToC relationship up front. We model exactly what "turn the page" needs, and no
  more. See [`glossary.md`](glossary.md) for the domain vocabulary the slices draw on.

## How the next slice is chosen

We **dogfood**: ship the smallest version worth opening to read, then read with it and let
the **next-most-annoying real problem** pick the next slice. The list below is a *seed
ordering*, **not a contract** — the real order emerges from use. See
[ADR-0002](adr/0002-dogfood-driven-prioritization.md).

## The slices (seed ordering)

Smallest-first; each is demoable on its own. The roadmap phase each slice feeds is noted, so
this vision and [`roadmap.md`](roadmap.md) stay in sync.

| # | Slice | A reader can… | Roadmap home | Learns (Rust/Dioxus) |
|---|-------|---------------|--------------|----------------------|
| S1 | **Open a book** | point the app at an `.epub` and see its **title** | M2 · epub-rendering | crate deps + Cargo features, `Result`/`?`, error at the edge |
| S2 | **Render a page** | see the **text** of the book's first content document | M2 · epub-rendering | spine access, rendering HTML in the webview |
| S3 | **Turn pages** | move **forward/back** through the book's documents | M2 · epub-rendering | signals holding position, event handlers, clamping |
| S4 | **See position** | know **where they are** ("3 of 12") | M2 · epub-rendering | derived state (`use_memo`) |
| S5 | **Remember the place** | **resume** where they left off after reopening | M2 · reading-position | `serde`, file I/O, a storage boundary (`dyn` trait) |
| S6 | **Open any book** | **pick** a file / choose from a small library | M2 · library | async (`use_resource`), file dialogs, list rendering |

Anything past S6 (styled book CSS + images, in-document pagination, search, annotations,
themes, mobile/web targets) is a later milestone — explicitly *not* MVP.

## Slice 1, grounded in the first book

First book: `book/The Adventures of Sherlock Holmes by Arthur Conan Doyle.epub` (Project
Gutenberg). Its spine is **16 items**: `[0]` an SVG cover, `[1]` the PG title/header,
`[2..=13]` the **twelve stories**, `[14]` the PG footer. The prose documents are pure
standard HTML (`<p>`, `<h2>`/`<h3>`, `<i>`) with **no inline images**, so dumped raw they
read fine on the webview's default styling.

**Slice 1:** open that hardcoded file, render the current spine item's XHTML raw (Dioxus
`dangerous_inner_html`) in a scrollable view, with **Next/Prev** walking the 16 items
(clamped at both ends).

- **Known-ugly, on purpose:** spine item 0 (the cover) renders as a *broken image* — its
  `cover.jpg` can't load without the asset protocol (a backlog unlock). You page past it
  (cover → header → Story 1). This is left raw deliberately: the first real annoyance it
  produces tells us what to build next (faithful images? skip-to-first-chapter? a ToC?).
- **Cheap to redo:** the raw `dangerous_inner_html` path is a contained swap when faithful
  rendering is pulled in — not a one-way door (ADR-0002).

## Backlog of unlocks (pulled in when they become the worst pain)

The author's eventual targets — but **demoted from MVP** to "pull in when reading actually
demands it" (ADR-0002). Each gets its own ADR when its slice arrives. Noted here, with the
one hard fact each hides, so they aren't forgotten:

- **Faithful styling** — the book's own CSS + images via a custom webview asset protocol
  (serve resources out of the zip). *(See [`../RESEARCH.md`](../RESEARCH.md).)* This is the
  **foundation** the next two build on, so if pulled in it likely comes first.
- **In-document pagination** — screen-sized pages (CSS multicolumn), not whole-document
  scroll. A page count is `content × CSS × viewport × font` — **ephemeral, not intrinsic**.
- **Reflow-stable position** — "the first word on the page," stored as a layout-independent
  anchor (EPUB CFI or a homemade DOM locator), *recomputed* into a page each session.
  Requires a JS↔Rust bridge in the webview. **A page number can never be the stored
  position.**

Their entanglement (all three are one rendering+pagination+position engine, with the build
order faithful → paginate → anchor) is why none is an MVP slice.
