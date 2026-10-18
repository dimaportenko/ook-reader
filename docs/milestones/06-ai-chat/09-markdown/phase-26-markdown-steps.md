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

## Step 1 — Formatted paragraphs

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

**What it is.** An assistant reply in the drawer shows `**bold**` as bold, `*italic*` as
italic, `` `code` `` as code and `~~struck~~` as struck through, with paragraphs spaced
apart. The bubble that is still streaming renders the same way. *Thinking…*, `...` and
every user message stay plain text.

### Runnable check first

`src/chat/markdown.rs` — eight tests on the pure `parse`, with two small helpers,
`text("…")` and `paragraph(vec![…])`:

```rust
#[test]
fn bold_nests_inside_italic() {
    assert_eq!(
        parse("*very **bold** claim*"),
        [paragraph(vec![Inline::Emphasis(vec![
            text("very "),
            Inline::Strong(vec![text("bold")]),
            text(" claim"),
        ])])]
    );
}

#[test]
fn line_breaks_are_kept() {
    assert_eq!(
        parse("one\ntwo  \nthree"),
        [paragraph(vec![
            text("one"),
            Inline::SoftBreak,
            text("two"),
            Inline::HardBreak,
            text("three"),
        ])]
    );
}

#[test]
fn an_unclosed_marker_mid_stream_stays_literal() {
    assert_eq!(parse("so **bol"), [paragraph(vec![text("so **bol")])]);
}
```

The others: `plain_text_is_one_paragraph`, `a_blank_line_starts_a_new_paragraph`,
`empty_text_has_no_blocks`, `inline_styles_wrap_their_text` (all four inline styles in one
line) and `unhandled_blocks_keep_their_text`. That last one feeds a heading, a list, a
quote, a fenced block, a table, a link, an image, inline HTML and a rule through `parse`,
and checks that every word of text still comes out somewhere in the paragraphs:

```rust
for expected in ["Title", "one", "two", "quoted", "let x = 1;", "a", "b", "c", "d",
                 "link", "alt", "<b>", "html", "</b>"] {
    assert!(seen.contains(expected), "{expected:?} was dropped from {seen:?}");
}
```

Red against a `parse` that returned `Vec::new()`: 7 failed, 236 passed, 3 ignored.
`empty_text_has_no_blocks` passed against the stub, as it should.

```
test chat::markdown::test::bold_nests_inside_italic ... FAILED
test chat::markdown::test::line_breaks_are_kept ... FAILED
test chat::markdown::test::inline_styles_wrap_their_text ... FAILED
test chat::markdown::test::plain_text_is_one_paragraph ... FAILED
test chat::markdown::test::an_unclosed_marker_mid_stream_stays_literal ... FAILED
test chat::markdown::test::a_blank_line_starts_a_new_paragraph ... FAILED
test chat::markdown::test::unhandled_blocks_keep_their_text ... FAILED
```

Green after: 243 passed, 3 ignored, and `cargo clippy --all-targets` was clean.

**Eyeball under `dx serve`:** ask something that gets a formatted answer ("Give me three
**bold** key points about this chapter"). While it streams, a half-typed `**` shows
literally and turns bold once it closes. Paragraphs have a gap between them, and the first
and last sit flush with the bubble's padding. Your own message still shows any `*` as
typed.

### Minimal implementation

- **`Cargo.toml`** — `pulldown-cmark = { version = "0.13", default-features = false }`
  (0.13.4 locked). Only the pull parser is used, so the `html` and `getopts` features stay
  off.
- **`src/chat/markdown.rs`** — `Block::Paragraph(Vec<Inline>)` and `Inline::{Text, Strong,
  Emphasis, Code, Strikethrough, SoftBreak, HardBreak}`. A private `Builder` holds the
  finished `blocks`, the current paragraph's `inlines`, and a stack, `spans: Vec<(Span,
  Vec<Inline>)>`, of styles still open. `parse` runs `Parser::new_ext` with
  `ENABLE_STRIKETHROUGH | ENABLE_TABLES` and matches each event:
  - `Start(Strong | Emphasis | Strikethrough)` pushes a frame; the matching `End` pops it,
    wraps its children (`Span::wrap`) and pushes the result into the frame below.
  - Every other `Start`/`End`, and `Rule`, is a block boundary: `end_block` closes any
    open spans and moves the inlines, if any, into a `Paragraph`.
  - `Start`/`End` of `Link` and `Image` are skipped, so their text flows into the
    paragraph around them.
  - `Text`, `Html`, `InlineHtml`, math and footnote references all push `Inline::Text`.
    `push` appends to the previous `Text` instead of starting a new one.
- **`src/ui/markdown.rs`** — `#[component] Markdown { text: String }` parses and renders
  each `Paragraph` as a `p`. A plain recursive `fn render_inlines(&[Inline]) -> Element`
  maps the inlines to `strong`, `em`, `code`, `del`, `br` and text. `SoftBreak` renders as
  `"\n"`.
- **`src/ui/chat.rs`** — `bubble` now holds an `Element` instead of a `&str`: the two
  placeholders as plain text, a reply in progress as `Markdown`. A finished turn renders
  `Markdown` when it's the assistant's and the plain text when it's the user's.
- **`src/ui/chat.css`** — `.chat_panel__turn p` gets `0.6em` top and bottom margins, and
  the first and last paragraph lose the outer one.

### Why it works

- **A stack turns a flat stream into a tree.** `pulldown-cmark` never hands you a tree. It
  says "strong starts", "text", "strong ends". Each open style is a frame on `spans`, and
  `push` always writes into the top frame. So `*a **b** c*` builds `Emphasis[a,
  Strong[b], c]` without any lookahead. The parser guarantees the events are well nested,
  so popping the top frame on any span `End` is always the right frame.
- **Degrading is a default, not a list.** `Event::Start(_) | Event::End(_)` catches every
  block tag this step doesn't know: headings, items, quotes, code blocks, table cells. Each
  one just ends the current paragraph. Text is only pushed in one place, so a new kind of
  event can't silently lose it. Steps 2–5 replace this catch-all one arm at a time.
- **`Event` is matched exhaustively.** No `_ =>` arm covers whole events, so if a later
  `pulldown-cmark` adds one, the build breaks instead of the text vanishing.
- **Merging adjacent text keeps the tree honest.** The parser splits text at characters it
  had to consider as markup. An unclosed `**bol` arrives as `Text("**")` then
  `Text("bol")`. Merging them gives one `Text("so **bol")`, so tests compare whole
  strings and the DOM gets one text node.
- **Mid-stream is free.** The streaming bubble re-renders on every delta, and `Markdown`
  re-parses the whole text each time. A lone `**` is literal text until its partner
  arrives, and then the next render makes it bold.
- **The bubble keeps `white-space: pre-wrap`.** That makes a `SoftBreak` rendered as
  `"\n"` show as a line break, the same as the raw text did. Degraded code blocks keep
  their line breaks too.

### Forks taken

- **`SoftBreak` shows as a line break, not a space.** CommonMark allows either. Models
  often put single newlines where they mean a new line, and the drawer showed them that
  way before this step. The alternative is to render it as `" "` and set `white-space:
  normal` on Markdown bubbles, which is what most web renderers do.
- **Unhandled blocks become paragraphs, not inlines.** A heading or a list item ends up as
  its own `Paragraph`, so the text keeps its line structure. A table becomes one paragraph
  per cell, which is ugly but loses nothing until Step 4.
- **Links and images flow inline.** Their label or alt text joins the surrounding
  paragraph, which is already the safe behaviour Step 5 keeps for unsafe links.
- **The bubble holds an `Element`.** The alternative is an enum for "placeholder or
  reply" that the RSX matches on. An `Element` is less code and keeps the `li` written
  once.
- **No wrapper element.** `Markdown` renders its `p`s straight into the `li`, so the CSS
  targets `.chat_panel__turn p`. A wrapper `div` with its own class would need a second
  CSS module or a shared one.
- **`pub(crate)`, not `pub`.** It matches `Conversation` next door; nothing leaves the
  crate.

### Scope note

- Headings, lists, quotes and rules are Step 2. Code blocks and `code` styling are Step 3.
  Tables are Step 4. Links, images and raw HTML are Step 5, and until then all of them
  degrade to plain text as above.
- `Options::ENABLE_TABLES` is on already, so the table text shows up as paragraphs now and
  Step 4 only has to add the variant.

### Review notes (from the `simplify` pass)

- **Applied:** renamed the stack field from `open` to `spans` so it doesn't share a name
  with the `open` method. Folded `TaskListMarker`, which can't occur without
  `ENABLE_TASKLISTS`, into the arm that ignores links and images.
- **Skipped:** reading `message.role()` once into a local. It's called twice in the same
  `li`, and a `let` inside the RSX `for` would be more lines than it saves.
