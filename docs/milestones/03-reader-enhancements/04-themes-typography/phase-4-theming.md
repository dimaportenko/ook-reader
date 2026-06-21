# Phase 4 — Themes & Typography

[← Feature: Themes & Typography](README.md) · **Status:** ⬜ planned ·
build log: [`phase-4-theming-steps.md`](phase-4-theming-steps.md) ·
decision: [ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md)

## Goal

Give the reader control of the book's appearance through an **injected override layer**
(Readium-CSS model): pick day / sepia / night, then adjust typography — while the book's own
CSS stays in place and keeps doing its structural job. First slice ships the three colour
themes; typography settings follow.

## The shape (from ADR-0003)

A theme is not "the book's CSS *or* ours" — it's a **cascade** with the book's CSS in the
middle:

```
RS defaults  (--RS__*,  injected BEFORE book CSS, loses to it)
   ↓
author CSS   (the book's own stylesheets, untouched)
   ↓
USER layer   (--USER__*, injected AFTER book CSS, wins)
```

Driving it needs an **injection seam**. We get one by rendering each content document as a
**served XHTML resource** (`Content-Type: application/xhtml+xml`, iframe `src="/epub/…"`)
instead of `srcdoc` — which also fixes the anchor-wrap bug (see below). A settings change is
then "re-serve / reload the frame," so the sandbox stays script-free.

## Planned steps

See the [build log](phase-4-theming-steps.md) for each step's runnable check → minimal
implementation → why. Smallest-first:

- [ ] **Step 1 — Serve the current content document as XHTML.** Swap iframe `srcdoc` for
      `src="/epub/…"`, serving the *rewritten* doc with `application/xhtml+xml`. Fixes the
      anchor-wrap rendering bug **and** establishes the injection seam.
- [ ] **Step 2 — Model a theme in Rust.** A `Theme` enum → a `:root { --USER__… }` CSS
      string. Pure Rust, `cargo test` on the variable values for day/sepia/night.
- [ ] **Step 3 — Inject the USER layer.** Wire Step 2's variable block + a minimal
      override sheet into the served document, *after* the book's CSS (via `rbook`'s
      `inject_css`, which writes before `</head>`). Night actually darkens the page.
- [ ] **Step 4 — Add the RS-defaults layer before the book CSS.** Completes the three-tier
      cascade (RS < author < USER). Watch the injection point — `inject_css` only writes at
      end-of-head, so the *before* layer needs our own head-rewrite (the spot ADR-0003 flags
      as the only realistic `rbook`-fork trigger).
- [ ] **Step 5 — Theme switcher in the app chrome.** A `use_signal` holds the current theme;
      Day/Sepia/Night controls re-serve/reload the iframe.
- [ ] **Step 6 — Typography settings (later).** font-size, line-height, line-length,
      margins, then font-family from a *curated* list — each a `--USER__*` variable + a
      control, sequenced one at a time.
- [ ] **Step 7 — Review & refactor** (per the repo's phase-ending convention).

## Known constraints (from research)

- **Layer, don't replace.** User settings win via `--USER__*` + minimal, scoped `!important`;
  the book's CSS keeps structural styling. Respect author intent — when fonts are embedded or
  the author uses `!important`, some user settings deliberately yield (Readium gates the
  aggressive ones behind flags). Don't recreate Kobo-style invisible-text bugs.
- **Themes are variable sets.** day/sepia/night = `--USER__backgroundColor` +
  `--USER__textColor` (+ image filters: `darkenImages` / `invertImages`). Caching and custom
  themes fall out of this for free.
- **Curated fonts, not a free picker.** font-family offers a small named list
  (old-style / modern / sans / humanist), matching reader conventions.
- **Settings-change = reload** (script-free sandbox). Acceptable for one small book; revisit
  if it bites.
- **Language-sensitive settings.** hyphenation / text-align don't apply to CJK; out of scope
  for the first English-only slice, noted so it isn't designed out.

## The anchor-wrap bug (fixed by Step 1)

Chapters render as one giant link that turns red on hover. Cause: the content doc is XHTML
with `<a id="chap01"/>` (self-closing, no `href`) and **no** `</a>`; `srcdoc` parses as HTML,
where `<a>` isn't void, so the anchor never closes and wraps the chapter, inheriting the
book's `a:link` / `a:hover`. Not an `rbook` bug. Step 1's served-XHTML render makes the
webview parse it as XML, honouring `<a/>`. (Cross-referenced from the
[Phase 3 doc](../../02-basic-reader/01-epub-rendering/phase-3-epub-rendering.md).)

## Reference

[Readium CSS — user settings & themes](https://readium.org/css/docs/CSS12-user_prefs.html) ·
[Readium CSS — variables API](https://readium.org/css/docs/CSS19-api.html) ·
[Readium CSS — user-settings recommendations](https://github.com/readium/css/blob/master/docs/CSS14-user_settings_recs.md) ·
[`rbook` rewrite/inject_css](https://docs.rs/rbook/latest/rbook/) ·
[ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md).
</content>
