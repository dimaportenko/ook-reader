# ADR-0002 — Dogfood-driven prioritization

**Status:** accepted · 2026-06-18 · refines [ADR-0001](0001-walking-skeleton-vertical-slices.md)

## Context

ADR-0001 committed to thin vertical slices but still implied a fixed slice *sequence*
(S1–S6) decided up front. When grilled on the hard trio — paginated pages, faithful styling,
reflow-stable position — the author initially picked the hard version of all three, then
stepped back: *"I would like to build the project driven by interest and as a consumer of
the app… as soon as I can start to read a book and use it somehow, that's the right way to
go. The next step should solve exactly the next most annoying problem or most desired
feature… each step unlocks the next."*

The author is both the builder and the primary user. That makes real usage the most
trustworthy signal for what to build next — more trustworthy than a pre-planned feature
order.

## Decision

Prioritize by **dogfooding**. Ship the smallest version the author would actually use to
read a book, then **read with it** and let the **next-most-annoying real-usage problem**
(or most-wanted feature) choose the next slice. Each shipped slice unlocks the next by
revealing what now hurts most.

Consequences for planning:

- The S1–S6 list in [`../vision-mvp-reader.md`](../vision-mvp-reader.md) is a **seed
  ordering**, not a contract. The real order emerges from use.
- The hard trio (faithful styling, in-document pagination, reflow-stable position) is
  **demoted** from "MVP requirements" to a **backlog of unlocks**, each pulled in when it
  becomes the thing that most blocks reading. None is built ahead of that pain.

## Consequences

- **Good:** fastest path to a usable reader; every step is motivated by felt need; no
  speculative engineering.
- **Cost / risk — local maxima:** always fixing the *immediate* annoyance can build a slice
  in a way that turns a *predictable future* annoyance into a painful rewrite (e.g. a raw
  render path that the faithful asset protocol later replaces wholesale). One-way-door
  decisions need flagging *before* the slice is sized.
- **Mitigation:** that flagging is exactly the job of the grilling sessions
  (`grill-with-docs`). Before sizing a slice we ask "what near-certain future need would
  this choice make expensive?" and record any such trade-off as its own ADR. Cheap-to-redo
  slices proceed without ceremony.
