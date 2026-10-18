# Phase 26 — Markdown replies

[← Feature: Markdown replies](README.md) · **Status:** ⬜ planned — captured 2026-10-18 ·
build log: [`phase-26-markdown-steps.md`](phase-26-markdown-steps.md)

## Goal

A reply in the chat drawer reads like formatted text. Bold and italic show as bold and
italic, lists as lists and code as monospaced blocks, and tables line up. This holds while
the reply is still streaming, not only once it has finished. The phase closes when a real
Luna or Gemini reply with a list, a code block and a table renders correctly in the drawer.

> **Captured 2026-10-18 through `lbb:feature`,** after a grilling round that settled the
> approach, the scope and the link behaviour (below).

## The crux

**Parse into your own types, then render those.** `pulldown-cmark` emits a flat stream of
`Start(Tag)` / `Text` / `End(TagEnd)` events. The tempting shortcut is `push_html` plus
`dangerous_inner_html`. That hands untrusted model output to the WebView as HTML, and every
link becomes something the app has to police after the fact. Instead, the events are
**folded into a small typed tree**: `Vec<Block>`, where each block holds `Inline`s. RSX then
renders that tree. That gives the phase its lessons:

1. **A flat event stream into a tree** is a stack: `Start` pushes a frame, `End` pops it and
   attaches the finished node to its parent. It's the same move as any SAX-to-DOM builder.
2. **A pure function is the testable core.** `parse(&str) -> Vec<Block>` needs no UI and no
   WebView, so every syntax rule gets a plain `#[test]`. The component is a thin `match` over
   the tree.
3. **Safety by construction.** The tree has no "raw HTML" variant, and a link holds a URL
   that has already been vetted. Nothing has to be sanitized, because nothing unsafe can be
   represented.

## Design decisions (recorded up front)

- **Typed tree, not an HTML string.** This was chosen in the grilling round over
  `push_html` + `dangerous_inner_html`.
- **Assistant messages only.** User messages carry the prefilled passage template and stay
  plain text.
- **CommonMark + strikethrough + tables.** Syntax highlighting, math, task lists,
  footnotes and a copy button are out of scope.
- **Re-parse every update while streaming.** No caching and no incremental parsing. A reply
  is a few KB and parsing it takes microseconds.
- **Links:** only `http`, `https` and `mailto` become `a { href }`, and dioxus's navigation
  handler opens them in the system browser (see the README's research). Any other link is
  shown as its text, not an anchor. **Images** render as their alt text, so the app never
  fetches a URL a model chose.
- **Raw HTML** in a reply (`Event::Html` / `InlineHtml`) is shown as literal text.
- **Styling** lives in `chat.css` and uses the existing theme variables, so light, dark and
  sepia all work. Wide code blocks and tables scroll sideways inside the bubble.
- **Where it lives** *(default, overrule in review)*: the parser goes in
  `src/chat/markdown.rs`, next to `Conversation`, because it's chat logic with no UI. The
  component goes in `src/ui/markdown.rs`. `pulldown-cmark = { version = "0.13",
  default-features = false }`.

## Planned steps

- [ ] 1. Formatted paragraphs — `parse` with paragraphs and inline bold, italic, code, strikethrough and line breaks; a `Markdown` component renders every assistant bubble, streaming included
- [ ] 2. Headings, lists, quotes and rules — the block structure, nested lists included, with its CSS
- [ ] 3. Code blocks — fenced and indented, the fence's language kept, sideways scroll, an unclosed fence rendered mid-stream
- [ ] 4. Tables — GFM tables with column alignment, scrolling sideways inside the bubble
- [ ] 5. Links, images and raw HTML — safe-scheme anchors that open externally, alt text for images, HTML as literal text
- [ ] 6. Review and refactor
