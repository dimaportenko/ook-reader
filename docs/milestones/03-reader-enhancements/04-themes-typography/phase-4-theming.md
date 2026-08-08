# Phase 4 — Themes & Typography

[← Feature: Themes & Typography](README.md) · **Status:** 🚧 in progress ·
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
handler that serves it is where we inject, so a chapter is born with the current settings
already in its bytes. Changing a setting *afterwards* does not go back through the handler —
see the decision on live updates below.

> **Prerequisite:** Phase 3 Step 8 (served-XHTML renderer) must land first. It also fixes the
> anchor-wrap rendering bug — that fix is rendering correctness and lives in Phase 3, not here.

## Planned steps

See the [build log](phase-4-theming-steps.md) for each step's runnable check → minimal
implementation → why. Smallest-first:

- [x] **Step 1 — Model a theme in Rust.** A `Theme` enum → a `:root { --USER__… }` CSS
      string. Pure Rust, `cargo test` on the variable values for day/sepia/night. Committed
      in `27e1d86` with Step 2 — it was never landed on its own, so the `dead_code` warning
      it was planned around never happened.
- [x] **Step 2 — Inject the USER layer.** Wire Step 1's variable block + a minimal
      override sheet into the served document, *after* the book's CSS **and after
      `pagination.css`**, which is the tie that actually mattered — it is ours and
      `!important` throughout. `rbook`'s `inject_css` cannot reach that slot (it writes
      during the rewrite, so `insert_before_head_close` always lands after it), so the theme
      is concatenated onto `INJECTED_ASSETS` instead. Committed in `27e1d86`, **54 tests
      green**.
- [x] **Step 3 — Add the RS-defaults layer before the book CSS.** Completes the three-tier
      cascade (RS < author < USER). `insert_after_head_open` is the sibling to
      `insert_before_head_close`, and the subtlety is that the opening tag has no single
      spelling — `<head`, plus a check that what follows is `>` or whitespace, because
      `<header>` would otherwise match. ADR-0003 flagged this as the only realistic
      `rbook`-fork trigger; it stayed closed, since Step 2 had already dropped `inject_css`
      and made head surgery ours. Committed in `1fe0398`, **58 tests green**.
- [x] **Step 4 — Theme switcher in the app chrome.** A `use_signal` holds the current theme;
      a Day/Sepia/Night `<select>` sits on both screens. The change reaches the document by
      **two routes**: the served bytes carry it for a chapter that has not loaded yet, and an
      `ook-set-theme` `postMessage` writes the `--USER__*` values as an inline style on the
      live frame's `documentElement` for one already on screen. **No reload** — a colour
      change is a repaint, so the layout, the page count and the page you are on all survive
      untouched. Committed in `ed1df0d`, **61 tests green**.
- [x] **Step 5a — A `Settings` struct owns the variable list.** Plumbing: `Theme` goes back
      to being a palette, `Settings` becomes the thing that has a theme and produces the
      `--USER__*` pairs. The pushed collection changes from
      `[(&'static str, &'static str); 2]` to `Vec<(&'static str, String)>` — the count stops
      being fixed and the values stop being literals, both of which 5b needs. The one visible
      change: the library screen gives up its theme picker, the reader keeps its own.
      Committed in `df7e4f0`, **62 tests green**.
- [ ] **Step 5b — `--USER__fontSize` + its control.** The first non-colour variable, end to
      end. Text resizes live; the page count goes stale, deliberately, so 5c has something
      you have watched break.
- [ ] **Step 5c — Re-measure and re-anchor after a reflow.** A size change re-columns the
      document, and `page-count.js` only re-reports on `resize`. Re-report the count and land
      back on the same *words*, via the `ook-sel:` selector `restored_data` already uses.
- [ ] **Step 5d — `--USER__lineHeight`, then margins / line-length.** Cheap once 5c exists.
      Line-length last of the three — it is the one that touches `pagination.css`'s
      `column-width`.
- [ ] **Step 5e — `--USER__fontFamily` from a curated list.** Respect embedded fonts and
      author `!important`, the way Readium gates its aggressive overrides.
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
- ~~**Settings-change = reload** (script-free sandbox). Acceptable for one small book;
  revisit if it bites.~~ **It bit, in Step 4.** Both halves of the premise had expired: the
  sandbox is not script-free (`allow-scripts`, plus seven injected assets), and the reload is
  not acceptable — it tears down the document, re-measures it and blanks the frame to change
  two colour values, which reads as a blink on every click. **Settings-change = message.**
  The reader posts the `--USER__*` name/value pairs to the frame and the document writes them
  onto `documentElement` as an inline style, which outranks the served `:root` block without
  anything being re-injected. Serve-time injection stays, for the first paint of a chapter
  that has not loaded yet. Colour changes need nothing further; Step 5's reflowing settings
  (font-size, line-length) still need to re-anchor the reading position afterwards, by
  selector rather than page number.
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
