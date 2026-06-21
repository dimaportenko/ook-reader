# Phase 3 — EPUB Rendering

[← Feature: EPUB Rendering](README.md) · **Status:** 🚧 in progress (Slice 1 underway) ·
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
- [ ] Fix the **anchor-wrap rendering bug** by rendering the current spine item as **served
      XHTML** (iframe `src="/epub/…"` with `Content-Type: application/xhtml+xml`) instead of
      `srcdoc`. Chapters currently render as a giant hover-red link because the XHTML
      self-closing `<a id="…"/>` is mis-parsed as unclosed under `srcdoc`'s HTML parser; an
      XML-parsed served document honours it (build log **Step 8**). This is a rendering-
      correctness fix that belongs here; the served-XHTML renderer it produces is also the
      seam [Phase 4 (theming)](../../03-reader-enhancements/04-themes-typography/phase-4-theming.md)
      builds on. See [ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md).
- [ ] Page turns: start with continuous vertical scroll; spike CSS multi-column +
      `translateX` for true pagination
- [ ] Intercept internal hyperlinks → navigation events (next/prev spine item)
- [ ] Bundle a small DRM-free sample `.epub` for testing

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
