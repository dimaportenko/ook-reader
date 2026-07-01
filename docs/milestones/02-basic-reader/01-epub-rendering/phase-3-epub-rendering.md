# Phase 3 — EPUB Rendering

[← Feature: EPUB Rendering](README.md) · **Status:** ✅ done ·
build log: [`phase-3-epub-rendering-steps.md`](phase-3-epub-rendering-steps.md)

## Goal

Add `rbook` and render a real `.epub`: open a file → read the spine → display a chapter's
XHTML in the webview, with working page turns.

> **Build order (ADR-0002, dogfood-driven).** This phase is approached as
> [Slice 1](../../../vision-mvp-reader.md) first: open the bundled book, render the current
> spine item **raw** (`dangerous_inner_html`), Next/Prev to walk it. The "Known constraints"
> below describe the *eventual faithful* renderer — the iframe + asset-protocol items are
> the deferred **faithful-styling unlock**, pulled in when a broken cover / missing styles
> become the worst real annoyance, not up front. Per-step detail in the build log.
>
> **Update (2026-06-20).** That unlock is being pulled forward by choice — the
> `use_asset_handler` custom protocol and sandboxed `<iframe>` items below are now sequenced
> as the **faithful-styling arc (Steps 4–7)** in the
> [build log](phase-3-epub-rendering-steps.md). The deferral is lifted deliberately, not by
> drift.

## Planned steps

- [x] Add `rbook` to `Cargo.toml` (`rbook = "0.7"`)
- [x] Open an `.epub` with `Epub::open`; read metadata, manifest, spine, TOC
- [x] Register a `use_asset_handler("epub", …)` that reads resource bytes out of the EPUB zip
      and `responder.respond(Response …)`. (No custom scheme / no `wry://` vs `http://wry.`
      split — dioxus routes by the request's first path segment on the app origin, so the URL
      is a plain root-relative `/epub/…`. See the build log's Step 5.)
- [x] Rewrite spine docs' OPF-relative URLs (images/CSS/fonts) to `/epub/…` — done by rbook's
      `EpubRewriteOptions` / `PathRewrite::prefix("/epub/")`, not hand-rolled (build log Step 6)
- [x] Render the current spine item in a **sandboxed `<iframe srcdoc>`** (omit `allow-scripts`)
      for style isolation (build log Step 7; needs the `current` signal from "Turn pages")
- [x] Fix the **anchor-wrap rendering bug** by getting the spine item in front of the browser's
      **XML** parser, so the self-closing `<a id="…"/>` is honoured instead of mis-parsed as an
      unclosed tag under `srcdoc`'s HTML parser (which makes chapters a giant hover-red link).
      The clean "served XHTML via iframe `src`" route is **blocked on macOS** by dioxus's
      navigation guard (it refuses `dioxus://` iframe navigations after first load), so the
      renderer uses a **`data:application/xhtml+xml` URL** instead (build log **Step 8**, split
      8a render / 8b subresources). This is a rendering-correctness fix that belongs here; the
      bytes we build for the `data:` URL are also the theming-injection seam
      [Phase 4 (theming)](../../03-reader-enhancements/04-themes-typography/phase-4-theming.md)
      builds on — a relocation of the seam ADR-0003 assumed lived in the served response. See
      [ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md).
- [x] Page turns: start with continuous vertical scroll; spike CSS multi-column +
      `translateX` for true pagination
- [x] Intercept internal hyperlinks → navigation events (next/prev spine item) — done as build-log
      Steps 11a/11b/11c (resolve targets in Rust → iframe↔Dioxus `postMessage` bridge → scroll to the
      `#fragment`); committed `bf70e44` / `4b895f6` / `3b5aee5`
- [x] Bundle a small DRM-free sample `.epub` for testing — done as build-log **Step 12**
      (rename to `pg1661-…`, `CARGO_MANIFEST_DIR`-anchored `BOOK`, `book/README.md`, existence
      test); committed `56f2af3`
- [x] Review & refactor the finished EPUB rendering phase — build-log **Step 13** (the
      phase-ending review-and-refactor pass); committed `2d63663`

## Known constraints (from research)

- Render in an **iframe** (not `dangerous_inner_html`) for style isolation — the book's
  CSS would otherwise leak into the app.
- Resource paths are **OPF-relative** — rewrite them or inject `<base href>`.
- **Pagination happens in the webview (CSS/JS)**, not in Rust; Rust holds nav state.
- **DRM-free EPUBs only** (no LCP/ADEPT).

## Reference

[`RESEARCH.md`](../../../../RESEARCH.md) §3 ·
[`use_asset_handler`](https://docs.rs/dioxus-desktop/latest/dioxus_desktop/fn.use_asset_handler.html) ·
[rbook](https://docs.rs/rbook/latest/rbook/) ·
[ReadiumCSS pagination](https://github.com/readium/css/blob/master/docs/CSS03-injection_and_pagination.md).
</content>
