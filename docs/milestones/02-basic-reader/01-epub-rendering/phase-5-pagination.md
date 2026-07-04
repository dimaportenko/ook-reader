# Phase 5 — Pagination (page count per chapter)

[← Feature: EPUB Rendering](README.md) · **Status:** ✅ done ·
build log: [`phase-5-pagination-steps.md`](phase-5-pagination-steps.md)

## Goal

Turn Phase 3's pagination **spike** into real pagination: know how many pages a chapter
breaks into, show `Page X / N`, and clamp the Page-Next button so it can't walk off the
end of the chapter into blank space.

> **Continues Phase 3.** [Phase 3, build-log Step 10](phase-3-epub-rendering-steps.md)
> landed CSS multi-column + `translateX(--ook-page * -100vw)` paging *deliberately with
> "no page-count clamp yet."* This phase is the completion of that spike. It's item #1 in
> [`TODO.md`](../../../../TODO.md) ("Page count per chapter"), pulled ahead of the roadmap's
> Feature 2 (Library) by choice — a real page count is a prerequisite for sane clamping, a
> progress indicator, and restoring reading position later.

> **The crux.** Page count is a **layout-derived** quantity — Rust can't compute it. How
> many columns a chapter breaks into depends on font metrics, viewport width, and the
> book's own CSS, all of which live in the webview. So the shape is: **measure in JS after
> layout → report it over the existing `postMessage` bridge → store it in a signal → use it
> to clamp and label.** Rust owns the decisions; the webview owns the measurement. Every
> seam this needs already exists (the `inject_*` pattern, the bridge, the `ook-scroll`
> handler that already sets `page` from a JS-reported number).

## Planned steps

*(Steps 2 and 3 were merged during the build — the clamp and the roll-over were built
together; see the [build log](phase-5-pagination-steps.md) for the as-built plan.)*

- [x] **Step 1 — Measure & report the page count.** Inject a `load`/`resize` probe that
      computes `ceil(body.scrollWidth / innerWidth)` and posts `{ kind: 'ook-pages', count }`;
      store it in a `page_count` signal; show `Page X of N`. Unit-test the injection seam;
      eyeball the number under `dx serve`.
- [x] **Step 2 — Clamp Page-Next and roll turns over chapter boundaries.** One pure `Nav`
      decider (`on_next`/`on_prev`) that clamps to the count *and* spills across chapter
      edges, with a `#[test]`. (Merges the originally-separate "clamp" and optional
      "roll-over" steps, since they were built together.)
- [x] **Step 3 — Review & refactor** the pagination phase (mandatory phase-closing pass) —
      consolidate the reader's signals into a `Copy` `ReaderState`. *(committed in `9787cf1`)*
- [x] **Step 4 — Model the inbound bridge protocol** as a `BridgeMsg` enum with a pure,
      unit-tested `parse`, leaving the future's loop as pure dispatch. *(committed in `6c619fd`)*
- [x] **Step 5 — Relocate** the nav cluster into `src/nav.rs` and land three glue tidies
      (`BRIDGE_JS` const, `use_bridge` hook, `epub::use_register_asset_handler`). *(committed
      in `a1f3aaf` + `6e2861f`)*

> **Phase closed.** All steps done, 14 tests green. Steps 4–5 were added to the plan during
> the build (as follow-on refactors after the Step-3 review) and are recorded in full in the
> [build log](phase-5-pagination-steps.md); this checklist was reconciled to match.

## Known constraints

- **Measure on `load`, not inline** — `document.body.scrollWidth` is only the full
  multi-column span *after* the browser has laid out the columns.
- **`--ook-page` is baked into the data URL**, so every page turn reloads the iframe and
  re-fires the probe. Harmless (same count reported) but redundant — decoupling page-turn
  from full reload is a candidate for a later refactor, not this phase.
