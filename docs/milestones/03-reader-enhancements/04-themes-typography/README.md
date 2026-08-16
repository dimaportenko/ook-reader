# Feature: Themes & Typography

[← Milestone 3: Reader Enhancements](../README.md)

**Outcome:** the reader (the human) controls how the book looks — day / sepia / night, then
font size, line height, line length, and margins — without the publisher's CSS getting in
the way, and without throwing that CSS away. **Status:** ✅ done — all six settings ship
(theme, font size, line height, page margins, max line length, font family), persist across
restarts, and re-anchor the reading position after every reflow.

## Why this, why now

Dogfooding the faithful-styling arc ([Phase 3](../../02-basic-reader/01-epub-rendering/phase-3-epub-rendering.md))
showed the book's own CSS dictating colours and type with no reader control. It also surfaced
a concrete rendering bug (chapters render as a giant hover-red link) whose fix —
rendering content as **served XHTML** — is rendering correctness and lands in
[Phase 3, Step 8](../../02-basic-reader/01-epub-rendering/phase-3-epub-rendering-steps.md), not
here; this feature *builds on* the corrected renderer. The decision to **layer** reader styles
over the book's CSS (rather than replace it), to render content as served XHTML, and to **not
fork `rbook`**, is [ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md).

> **Sequencing, as it actually went.** This feature stays in Milestone 3 (it *is* a reader
> enhancement). Its plan was written early with the intent of working it **ahead** of
> Milestone 2's Library and Reading-Position features (ADR-0002). That is not what happened:
> Library (Phase 6) and Reading Position (Phase 7) shipped first, closing Milestone 2, and
> this phase is picked up after them. The plan below needed no revision for the delay — but
> Phase 4 now inherits a served-XHTML renderer that has since grown injected assets,
> pagination, and a position bridge, so its injection seam has more neighbours than the plan
> assumes.

> **A defect this phase surfaced, fixed elsewhere.** Same shape as the rendering bug above,
> and filed by the same rule. Dogfooding the typography work showed every book reopening
> **one page behind** where it was closed — because `FontFamily::Publisher`, the default,
> deliberately leaves the stack empty so the book's own `@font-face` files apply, and those
> load *after* the `load` event that position restore measures on. The cause is typography;
> the deliverable is that reopening lands where you stopped, so the fix and the investigation
> live with Reading Position:
> [Position across a reflow](../../02-basic-reader/03-reading-position/position-across-a-reflow.md).

## Phases

| # | Phase | Outcome | Status |
|---|-------|---------|--------|
| 4 | [Themes & Typography](phase-4-theming.md) | Readium-style injected override layer: day/sepia/night first, typography next | ✅ |

## Reference

[Readium CSS — user settings & themes](https://readium.org/css/docs/CSS12-user_prefs.html) ·
[Readium CSS — fundamentals (cascade & prefixes)](https://readium.org/readium-css/docs/CSS01-readiumcss_fundamentals.html) ·
[ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md) ·
[Glossary](../../../glossary.md) (theming terms).
</content>
