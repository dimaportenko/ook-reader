# ADR-0001 — Build the reader as thin vertical slices (walking skeleton)

**Status:** accepted · 2026-06-18

## Context

`ook-reader` is both a real EPUB reader and a vehicle for learning Rust + Dioxus by hand.
The initial plan sequenced work as foundational concepts first — a navigation/domain model
(spine vs ToC), generic state exercises (a counter, a `use_memo` doubling demo) — before any
reader feature existed. In review the author pushed back: those steps felt abstract and
disconnected from real usage ("I don't understand how I will use it"), and asked for
"really practical steps which I can use… minimal but valuable for real usage."

## Decision

Build the reader as a **walking skeleton**: a sequence of thin **vertical slices**, each one
end-to-end and usable by a real reader, smallest-first. Infrastructure and domain modelling
are pulled in by the first slice that needs them, never built ahead of a use. The
fundamentals-learning track is folded *into* the slices rather than run as separate
exercises.

The MVP slice sequence (S1–S6) and what each teaches is recorded in
[`../vision-mvp-reader.md`](../vision-mvp-reader.md).

## Consequences

- **Good:** every step produces visible value and code that ships; motivation and dogfooding
  stay high; each concept is learned in a context where its purpose is obvious.
- **Good:** hard domain decisions (the spine/ToC relationship, position encoding) are faced
  only when a concrete slice forces them, with real requirements in hand instead of guesses.
- **Cost:** some early code is deliberately partial (e.g. raw unstyled rendering before a
  proper asset pipeline) and gets revisited. We accept rework as the price of always having a
  working, usable app. Slice-forced decisions are captured in their own ADRs as they land.
- **Tension to watch:** "minimal slice" can hide a genuinely hard sub-problem (pagination,
  position stability). The grilling sessions exist to surface those before a slice is sized.
