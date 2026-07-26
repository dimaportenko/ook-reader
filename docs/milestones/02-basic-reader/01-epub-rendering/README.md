# Feature: EPUB Rendering

[← Milestone 2](../README.md)

**Goal:** parse an EPUB with `rbook` and display a chapter's content in the webview,
with paging — the core rendering loop everything else builds on.

**Status:** ✅ done

## Phases

| Phase | Outcome | Status |
|---|---|---|
| [Phase 3 — EPUB Rendering](phase-3-epub-rendering.md) | Open an `.epub` → render a spine item → turn pages | ✅ |
| [Phase 5 — Pagination](phase-5-pagination.md) | Page count per chapter → `Page X / N` → clamp Page-Next | ✅ |

## Rework — chapter transport

How a chapter reaches the iframe was rebuilt after the phases above shipped, driven by
findings #1 and #2 of
[`performance-review-2026-07.md`](../performance-review-2026-07.md). Read in order:

| # | Document | Outcome | Status |
|---|---|---|---|
| 1 | [Fragment scroll via the URL hash](fragment-scroll-via-url-hash.md) | Fragment moves out of an interpolated script into the URL hash | ✅ `30e4b0c` |
| 2 | [Serve chapters through the asset handler](serve-chapters-through-the-asset-handler.md) | Chapter bytes leave the VDOM; the iframe gets a short URL, fetched to a blob | 🚧 steps 1–4 + 3b shipped |
| 3 | [Review fixes](serve-chapters-review-fixes.md) | Review of #2 as shipped — the refetch bounce, percent-decoding, blob lifetime | ✅ §1–§7 |

## Reference

EPUB layer evaluation (parser choice, iframe rendering, resource serving, pagination):
[`RESEARCH.md`](../../../../RESEARCH.md) §3.
</content>
