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

Driving it needs an **injection seam**. We already have one:
[Phase 3, Step 8](../../02-basic-reader/01-epub-rendering/phase-3-epub-rendering-steps.md)
renders each content document as a **served XHTML resource**
(`Content-Type: application/xhtml+xml`, iframe `src="/epub/…"`) instead of `srcdoc` — the
handler that serves it is where we inject. A settings change is then "re-serve / reload the
frame," so the sandbox stays script-free.

> **Prerequisite:** Phase 3 Step 8 (served-XHTML renderer) must land first. It also fixes the
> anchor-wrap rendering bug — that fix is rendering correctness and lives in Phase 3, not here.

## Planned steps

See the [build log](phase-4-theming-steps.md) for each step's runnable check → minimal
implementation → why. Smallest-first:

- [ ] **Step 1 — Model a theme in Rust.** A `Theme` enum → a `:root { --USER__… }` CSS
      string. Pure Rust, `cargo test` on the variable values for day/sepia/night.
- [ ] **Step 2 — Inject the USER layer.** Wire Step 1's variable block + a minimal
      override sheet into the served document, *after* the book's CSS (via `rbook`'s
      `inject_css`, which writes before `</head>`). Night actually darkens the page.
- [ ] **Step 3 — Add the RS-defaults layer before the book CSS.** Completes the three-tier
      cascade (RS < author < USER). Watch the injection point — `inject_css` only writes at
      end-of-head, so the *before* layer needs our own head-rewrite (the spot ADR-0003 flags
      as the only realistic `rbook`-fork trigger).
- [ ] **Step 4 — Theme switcher in the app chrome.** A `use_signal` holds the current theme;
      Day/Sepia/Night controls re-serve/reload the iframe.
- [ ] **Step 5 — Typography settings (later).** font-size, line-height, line-length,
      margins, then font-family from a *curated* list — each a `--USER__*` variable + a
      control, sequenced one at a time.
- [ ] **Step 6 — Review & refactor** (per the repo's phase-ending convention).

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

## The anchor-wrap bug (fixed in Phase 3, not here)

The served-XHTML renderer this phase depends on is the same change that fixes the anchor-wrap
bug (chapters rendering as a giant hover-red link, because `srcdoc`'s HTML parser mis-reads the
XHTML self-closing `<a id="…"/>` as unclosed). That is a **rendering-correctness** fix, so it
lives in
[Phase 3, Step 8](../../02-basic-reader/01-epub-rendering/phase-3-epub-rendering-steps.md) —
Phase 4 simply builds on the corrected renderer. (Recorded in
[ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md).)

## Reference

[Readium CSS — user settings & themes](https://readium.org/css/docs/CSS12-user_prefs.html) ·
[Readium CSS — variables API](https://readium.org/css/docs/CSS19-api.html) ·
[Readium CSS — user-settings recommendations](https://github.com/readium/css/blob/master/docs/CSS14-user_settings_recs.md) ·
[`rbook` rewrite/inject_css](https://docs.rs/rbook/latest/rbook/) ·
[ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md).
</content>
