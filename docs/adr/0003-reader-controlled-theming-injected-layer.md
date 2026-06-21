# ADR-0003 — Reader-controlled theming via an injected override layer

**Status:** accepted · 2026-06-21 · builds on [ADR-0002](0002-dogfood-driven-prioritization.md) ·
realised by [Phase 4 — Themes & Typography](../milestones/03-reader-enhancements/04-themes-typography/phase-4-theming.md)

## Context

The faithful-styling arc (Phase 3, Steps 4–7) made the reader render the book's **own**
CSS inside a sandboxed `<iframe srcdoc>`. Dogfooding it surfaced two problems:

1. **The book's CSS fights the reader.** A Project Gutenberg book ships `a:link { color: blue }`,
   `a:hover { color: red }`, a fixed black-on-white body, drop-cap sizing, etc. There is no
   way for the *reader* (the human) to pick a dark or sepia mode, change the font size, or
   widen the margins — the page is whatever the publisher decided. That is the gap "add theme
   configuration as regular ebook-reader functionality" names.
2. **A concrete rendering bug:** chapters render as one giant link that turns red on hover.
   Root cause (confirmed against the bundled book and `rbook 0.7.9` source): the content
   document is **XHTML** with a self-closing fragment anchor `<a id="chap01"/>` and **no**
   `</a>`. `srcdoc` is parsed as **HTML (`text/html`)**, where `<a>` is *not* a void element,
   so `/>` is ignored, the anchor never closes, and the rest of the chapter becomes its
   descendant — inheriting `a:link` / `a:hover`. `rbook` is **not** at fault: its rewriter
   only rewrites attributes and (optionally) injects CSS into `<head>`; it round-trips `<a/>`
   faithfully. The breakage is at *our* rendering seam — XHTML fed to an HTML parser.

The initial framing was "using the book's CSS wasn't a good idea — replace it." Research into
the de-facto standard ([Readium CSS](https://github.com/readium/readium-css)) contradicted
that framing: mature readers do **not** discard the book's CSS. They **layer** reader styles
around it in a controlled cascade so the book keeps its structural styling (drop caps, verse
indentation, cover layout) while the reader owns theme and typography.

## Decision

Three linked decisions.

### 1. Theme by **layering**, not replacing — the Readium cascade

Drive reading settings through CSS **custom properties** (`--USER__*`) and a small injected
stylesheet, leaving the book's own CSS in place. The cascade has three tiers, by source
order and prefix convention:

- **RS defaults** — a normalize/defaults sheet injected **before** the book's CSS; its
  variables use the `--RS__` prefix and *lose* to the book.
- **Author/publisher CSS** — the book's own stylesheets, **untouched**, in the middle.
- **User overrides** — a sheet injected **after** the book's CSS; its `--USER__*` variables
  *win* (later source order + minimal, scoped `!important`).

Cascade priority by design: **USER > author > RS**. A **theme** (day / sepia / night) is just
a named set of `--USER__*` values — primarily `--USER__backgroundColor` and
`--USER__textColor`, plus image filters. Typography (font-size, line-height, line-length,
margins, font-family) is the same mechanism, more variables.

### 2. Render each content document as **served XHTML**, not `srcdoc`

Stop feeding the document to `iframe { srcdoc }` (HTML parse). Instead serve the **rewritten**
content document through the existing `/epub/` asset handler with
`Content-Type: application/xhtml+xml`, and point the iframe at `src="/epub/…"`. The webview's
**XML** parser then honours `<a/>` self-closing → the wrapper bug disappears at the parser
level. This also gives us a real document URL and a **serve-time injection point** for the
cascade sheets above — and a settings change becomes "re-serve / reload the frame," so we
keep the sandbox script-free (no `allow-scripts`). Building the injected document string is
pure Rust, so it stays on the testable side of the Rust/UI seam (like `load_spine`).

### 3. Do **not** fork `rbook` for this

The anchor bug and the theming layer both live at *our* rendering/injection seam, not inside
`rbook`'s parse or rewrite. `rbook`'s `inject_css` (injects a `<style>` just before `</head>`,
i.e. *after* the book's `<link>`s) is exactly the hook for the **USER-after** layer. A fork is
reserved for a future need we don't have yet — most plausibly injecting the **RS-before**
layer at the *start* of `<head>` (which `inject_css` can't do) if doing it in our own
head-rewrite proves awkward. A local clone already exists at `~/work/github/rbook`, so the
option stays cheap.

## Consequences

- **Good:** matches the de-facto standard; the book keeps its structural styling; one
  mechanism (CSS variables) scales from "dark mode" to full typography controls; the bug is
  fixed at the parser level rather than papered over; the injection seam is pure-Rust
  testable; the sandbox stays script-free.
- **Cost / complexity:** layering is *more* work than replacing — we maintain the cascade and
  must respect author intent (embedded fonts, `!important`) so we don't recreate Kobo-style
  invisible-text bugs. Settings-change-by-reload re-parses the document each time; fine for
  one ~380 KB book, a known cost to revisit if it bites.
- **One-way-door check (ADR-0002):** moving off `srcdoc` to served XHTML is a deliberate
  rewrite of the Phase-3 render path, recorded here rather than drifted into. The RS-before
  injection point is the spot most likely to reopen the `rbook` fork question; flagged so it
  is a decision, not a surprise.

## Alternatives considered

- **Replace the book's CSS wholesale.** Simpler, total theme control immediately — but loses
  the book's structural/semantic styling and throws away the faithful-styling arc just built.
  Rejected.
- **Keep `srcdoc`; expand self-closing non-void tags** (`<a …/>` → `<a …></a>`) before
  rendering. Fixes the visible bug with a small change, but is whack-a-mole (other XHTML/HTML
  void-element mismatches recur) and gives theming no injection seam. Rejected as the primary
  path; kept as a fallback note in the build log.
- **`allow-scripts` + live variable mutation** (the literal Readium approach). Fast, no
  reload — but reopens the sandbox we closed in Phase 3 Step 7, and `allow-scripts` +
  `allow-same-origin` together can let content defeat its own sandbox. Deferred unless
  reload-on-change proves too slow.
</content>
</invoke>
