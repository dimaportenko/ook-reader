# Feature: Themes & Typography

[← Milestone 3: Reader Enhancements](../README.md)

**Outcome:** the reader (the human) controls how the book looks — day / sepia / night, then
font size, line height, line length, and margins — without the publisher's CSS getting in
the way, and without throwing that CSS away. **Status:** ⬜ planned (full plan written).

## Why this, why now

Dogfooding the faithful-styling arc ([Phase 3](../../02-basic-reader/01-epub-rendering/phase-3-epub-rendering.md))
showed the book's own CSS dictating colours and type with no reader control. It also surfaced
a concrete rendering bug (chapters render as a giant hover-red link) whose fix —
rendering content as **served XHTML** — is rendering correctness and lands in
[Phase 3, Step 8](../../02-basic-reader/01-epub-rendering/phase-3-epub-rendering-steps.md), not
here; this feature *builds on* the corrected renderer. The decision to **layer** reader styles
over the book's CSS (rather than replace it), to render content as served XHTML, and to **not
fork `rbook`**, is [ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md).

> **Sequencing.** This feature stays in Milestone 3 (it *is* a reader enhancement), but its
> plan is written now and it is worked **next**, ahead of Milestone 2's Library and
> Reading-Position features. That is a deliberate choice, not drift (ADR-0002).

## Phases

| # | Phase | Outcome | Status |
|---|-------|---------|--------|
| 4 | [Themes & Typography](phase-4-theming.md) | Readium-style injected override layer: day/sepia/night first, typography next | ⬜ |

## Reference

[Readium CSS — user settings & themes](https://readium.org/css/docs/CSS12-user_prefs.html) ·
[Readium CSS — fundamentals (cascade & prefixes)](https://readium.org/readium-css/docs/CSS01-readiumcss_fundamentals.html) ·
[ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md) ·
[Glossary](../../../glossary.md) (theming terms).
</content>
