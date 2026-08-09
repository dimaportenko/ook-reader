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
- [x] **Step 5b — `--USER__fontSize` + its control.** The first non-colour variable, end to
      end. Text resizes live; the page count goes stale, deliberately, so 5c has something
      you have watched break. The size goes on `html` and the colours stay on `body` —
      moving them together looked like tidying and would have cost the colour layer its
      `!important`. Committed in `1529fc4`, **65 tests green**.
- [x] **Step 5c — Re-measure and re-anchor after a reflow.** A size change re-columns the
      document, and `page-count.js` only re-reported on `resize`. The `ook-set-theme` handler
      now captures an anchor selector *before* applying the variables, re-reports the count,
      and posts a new `ook-reflow` message when the anchor's column moved — so the count is
      right and you land back on the same *words*. Not `ook-scroll`, which also clears
      `Pending::Fragment` and would strand a mid-settle restore. Committed in `2a9e181`,
      **67 tests green**.
- [x] **Step 5d — `--USER__lineHeight`.** A field, a rule, a control — 5c's handler re-anchored
      it for free, with no new message and no new JS. The two traps were the unit (unitless,
      not `%`, or headings inherit a length and overlap) and the selector (`body *`, because
      inheritance loses to any author rule on the paragraph). A third one showed up in review:
      folding the leading into the colour rule widens an `!important` `background` shorthand
      onto every element, so the layer is three rules and has a tripwire saying why. Committed
      in `eb92bb6`, **72 tests green**.
- [x] **Interlude — the settings popover.** Not a planned step and not a cascade change: the
      three controls moved behind a gear button built on `dioxus-primitives`. Recorded so the
      log has no gap. Committed in `b0e10db`, **72 tests green** (unchanged).
- [x] **Step 5e — `--USER__pageMargins`.** The first setting that changes page *geometry*, and
      the first whose value is read by a stylesheet that does not declare it. `pagination.css`
      stops hard-coding `24px`/`48px` and derives padding, `column-width` and `column-gap` from
      one `--ook-column`, so that a column plus a gap keeps advancing exactly `100vw` — the
      step the transform moves by and the unit `pageOf` divides by. The number splits in two:
      `--RS__pageGutter` says how wide a comfortable margin is, `--USER__pageMargins` is the
      bare factor the reader scales it by. No new JS — 5c's handler re-anchored it, for the
      third time. Committed in `18b42c2`, **75 tests green**.
- [x] **Step 5f — `--USER__maxLineLength`.** Caps the measure with a `min()` inside
      `--ook-column` — one line, because 5e made padding and gap the leftover, so a smaller
      column widens both and still advances `100vw`. Answers the wide-desktop case a bare
      margin factor cannot: `--USER__pageMargins` scales a fixed 24px gutter, so it cannot
      reach a 1300px column. ~~its real content is the unit (`ch` couples it to
      `--USER__fontSize`, `rem` does not)~~ — **that premise was wrong.** `rem` *is* the root
      font-size, which is exactly what `--USER__fontSize` sets, so both units track it. `ch`
      wins for a different reason: it is the width of a `0` in the element's font, so it also
      tracks the font *family* 5g is about to make settable, holding the measure constant in
      characters under both settings. Committed in `fb7304f`, **78 tests green**.
- [x] **Step 5g — `--USER__fontFamily` from a curated list.** Four fallback chains, each
      ending in a generic family — the only link that cannot miss, since a stack whose named
      faces are all absent resolves to the UA default, i.e. the font the book was already
      showing. Lands on `body` and its descendants with the monospace elements excluded,
      because their font is structural. **Carried 5f's loose end:** the on-screen check found
      no need to add `html` to the selector, so `ch` resolving at the *use* site — and with it
      5f's reason for preferring `ch` over `rem` — stands unrefuted. Committed in `cb03a4b`
      with 5h, **89 tests green** (83 at 5g alone).
- [x] **Step 5h — Respect the publisher's font.** Split out of the plan's single 5g, which was
      carrying two ideas: a stack is a value like every setting before it, but *not
      overriding* is a rule that has to appear and disappear, and a variable push cannot carry
      a rule. Readium's gate — `:root[style*='--USER__fontFamily']`, matching only when the
      push has written the property onto the root's inline style — is the answer, and the
      thing it exposes is that serve-time injection writes a `:root` **rule** while the gate
      reads the `style` **attribute**. `bootstrap_js()` closes that by writing the stack onto
      `documentElement` from the head, so both routes land in the same place. First step in
      the phase to need new JavaScript: an empty pushed value now means `removeProperty`,
      because `setProperty(name, "")` is a no-op that would weld the gate open. Committed in
      `cb03a4b`, **89 tests green**.
- [x] **Step 6 — Split the data layer.** Not a theming step, and it goes *before* persistence
      because of what planning persistence exposed: `Library` owns a SQLite connection **and** a
      managed-file directory while being named after neither, which is why `list` is five lines
      and `add_from_path` is seventy. Settings need the first and have nothing to do with the
      second. Extract `Db` (connection + schema, queries as `impl Db` blocks in child modules so
      `conn` stays private to `db/`) and `BookFiles` (the directory, copy, cover, unlink), leaving
      `Library` as the facade whose only remaining logic is the two-resource rollback — and whose
      public API does not change, so no UI file moves. A pure refactor: the check is that the
      same **89 tests** pass before and after, with no new ones and no changed count.
      **Landed at 90, not 89** — 6b's relocated `updated_at` assertion arrived in a module with no
      test to join and became one. That is the whole of the drift across three sub-steps: one test
      renamed and rehoused, no assertion added, no behavior changed. `Library` is a facade,
      `library/mod.rs` is 531 lines where it was 632, and `db/` holds the connection, the schema
      and both entities.
  - [x] **6a** — extract `BookFiles`; touches no SQL, so it separates cleanly and goes first.
        `write_cover` takes an extension and bytes rather than `epub::CoverImage`, so the file
        store never imports `crate::epub`. Committed in `75aaf71`, **89 tests green** — the same
        89, none edited, none added.
  - [x] **6b** — introduce `Db`, move `positions` onto it; the small entity proves the layout.
        It proved more than that: privacy made the layout real (`db/positions.rs` reaches
        `self.conn` as a *descendant* of the module declaring `Db`, nothing else can), and it
        forced a scaffold the plan had missed — the five books queries still in `Library` cannot
        reach a private field from a sibling module, so `Db::conn()` exists until 6c deletes it.
        Committed in `ec3196e`, **90 tests green** — 89 plus the `updated_at` assertion, which
        became its own test on arriving in a module that had none.
  - [x] **6c** — move `books` onto `Db`; **zero** test edits expected, because the books tests
        already go through `Library`'s public API. `add_from_path` gets shorter and finally reads
        as what it is: acquire, acquire, commit, or unwind. The prediction held exactly: not one
        assertion moved, `library/mod.rs` lost 101 lines, and `Db::conn()` — 6b's scaffold —
        is gone, which is the completion check no test count could make. Committed in `cb8f2af`,
        **90 tests green**, with the `dx serve` walk for the whole of Step 6 confirmed.
- [ ] **Step 7 — Persist the settings.** The deferral every step since 4 has been logging, now
      landing in the module Step 6 built for it.
  - [x] **7a** — a `settings` table on `Db`: one typed row (`CHECK (id = 1)`), an upsert and a
        `query_row`, provable entirely by `cargo test` with nothing wired up. Stores the struct's
        fields, not `css_vars()`'s rendered strings — `125` is the state, `"125%"` is the
        rendering — and enum slugs rather than discriminants, so reordering a variant cannot
        re-map stored values. Its tests need a `Db` and no `Library` at all, which is the
        extraction paying for itself on first use — and the slug-corruption test needs `db.conn`,
        which only a test inside `db/` can reach, so the extraction bought the *third* test
        outright. Committed in `500cacd`, **93 tests green**.
  - [ ] **7b** — load before the first paint, save on every change. All about order: `library`
        moves above `settings` in `App` and the signal is *born* holding the stored value, because
        applying it from an effect afterwards is a visible flash. One `use_effect` reading
        `settings()` covers all six controls — and writes the row back to itself on mount, which
        is harmless but worth knowing. `OrLog` widens to `E: Display` here.
- [ ] **Step 8 — Review & refactor** (per the repo's phase-ending convention). Lighter on module
      organization than usual, since Step 6 spent it; heavier on the four questions Step 6
      deliberately deferred.

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
