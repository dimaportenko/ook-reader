# Phase 26 — Markdown replies — build log

[← Phase 26](phase-26-markdown.md) · the phase doc holds the goal, decisions and the step
index; this file holds the test → code → why for each step, newest at the bottom.

## The crux

`pulldown-cmark`'s flat `Start`/`Text`/`End` events are folded, with a stack, into a typed
`Vec<Block>` / `Inline` tree that has no raw-HTML node. A pure `parse(&str)` is the tested
core, and a thin `Markdown` component renders the tree as RSX in every assistant bubble,
the streaming one included.

## Step plan

1. **Formatted paragraphs.** Add `pulldown-cmark` (no default features) and
   `src/chat/markdown.rs` with `parse(&str) -> Vec<Block>`. It covers `Block::Paragraph`
   and the inlines `Text`, `Strong`, `Emphasis`, `Code`, `Strikethrough`, `SoftBreak` and
   `HardBreak`, nested (bold inside italic). Unhandled events degrade to their text, never
   panic and never vanish. A `Markdown { text }` component in `src/ui/markdown.rs` replaces
   the raw `"{message.text()}"` for assistant turns and for the streaming bubble's reply
   text. *Thinking…* and `...` stay plain.
2. **Headings, lists, quotes and rules.** `Block::Heading`, `List { start: Option<u64>,
   items }` with nested lists and tight/loose items, `Quote(Vec<Block>)` and `Rule`. Add CSS
   for heading sizes inside a bubble, list indentation and the quote's border.
3. **Code blocks.** `Block::Code { lang: Option<String>, text }` for both fenced and
   indented blocks, plus styling for inline `code`. Use a monospaced font, a theme-tinted
   background, and `overflow-x: auto` so long lines scroll. A test pins that an unclosed
   fence, as seen mid-stream, still yields a code block that holds the partial text.
4. **Tables.** `Block::Table { align, head, rows }` with each cell holding inlines, from
   `Options::ENABLE_TABLES`. It renders as a real `table` inside a wrapper that scrolls
   sideways.
5. **Links, images and raw HTML.** `Inline::Link { href, children }` only when the URL's
   scheme is `http`, `https` or `mailto` (case-insensitive). Any other link keeps just its
   children. Images become their alt text. `Html` / `InlineHtml` become literal text. Tests
   cover `javascript:`, a relative href and a `<script>` tag.
6. **Review and refactor.** Run a `code-review` pass over the phase, apply the deferred
   cleanups, and close the phase. Hand checks still pending stay unconfirmed.

**Why this order.** Each step ends with something you can see in the drawer:

- **Step 1** carries the plumbing: the dependency, the tree, the component, and wiring it
  into both kinds of bubble. It deliberately does this with the smallest syntax. Once it
  lands, every later step is "add a variant, a test and a `match` arm".
- **Steps 2–4** add block structure from the most common in chat replies (lists) to the
  rarest (tables). Code blocks come before tables because they carry the streaming
  edge case.
- **Step 5** is last among the features because the safety rules need the full tree to
  exist first. Until then, links and images degrade to text, which is already safe.
- **Step 6** reviews the whole phase at once.

Step details are written by `lbb:next-implement` as each step is built.
