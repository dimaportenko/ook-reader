# Phase 26 — Markdown replies — build log

[← Phase 26](phase-26-markdown.md) · the phase doc holds the goal, decisions and the step
index; this file holds the test → code → why for each step, newest at the bottom.

## The crux

`pulldown-cmark`'s flat `Start`/`Text`/`End` events are folded, with a stack, into a typed
`Vec<Block>` / `Inline` tree that has no raw-HTML node. A pure `parse(&str)` is the tested
core, and a thin `Markdown` component renders the tree as RSX in every assistant bubble,
the streaming one included.

## Step plan

1. ~~**Formatted paragraphs.**~~ Add `pulldown-cmark` (no default features) and
   `src/chat/markdown.rs` with `parse(&str) -> Vec<Block>`. It covers `Block::Paragraph`
   and the inlines `Text`, `Strong`, `Emphasis`, `Code`, `Strikethrough`, `SoftBreak` and
   `HardBreak`, nested (bold inside italic). Unhandled events degrade to their text, never
   panic and never vanish. A `Markdown { text }` component in `src/ui/markdown.rs` replaces
   the raw `"{message.text()}"` for assistant turns and for the streaming bubble's reply
   text. *Thinking…* and `...` stay plain. **Done** — `781f5e0`.
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

> **Status:** done — committed in `781f5e0` (243 tests green, 3 ignored; clippy clean). The drawer rendering is still to be confirmed by hand under `dx serve`.

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

## Step 2 — Headings, lists, quotes and rules

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

**What it is.** A reply with `## Steps`, a bulleted or numbered list (nested lists
included), a `> quote` or a `---` rule now shows a heading, a real list, an indented quote
with a bar on its left, and a thin line. A numbered list that starts at 3 still counts from
3. Code blocks and tables still show as plain paragraphs, and links and images still show
as their text.

### The crux of this step: a second stack, for blocks

Step 1 had one stack, `spans`, for inline styles that are still open, and every finished
paragraph went straight into the flat `blocks`. That works only while blocks never contain
blocks. A list item holds paragraphs and maybe another list; a quote holds paragraphs and
lists. So blocks now need the same trick inlines got: a stack of **containers** still
open. `Start(List)` or `Start(BlockQuote)` pushes a frame, every finished block goes into
the top frame instead of `blocks`, and the matching `End` pops the frame, turns it into a
`Block` and pushes that into the frame below. The two stacks stay separate because they
nest at different levels: a span can never hold a block, and a block always ends every
span still open inside it.

### Runnable check first

`src/chat/markdown.rs` — seven new tests, and one reworked:

```rust
#[test]
fn lists_nest_inside_items() {
    assert_eq!(
        parse("- one\n  - inner\n- two"),
        [Block::List {
            start: None,
            items: vec![
                vec![
                    paragraph(vec![text("one")]),
                    Block::List {
                        start: None,
                        items: vec![vec![paragraph(vec![text("inner")])]],
                    },
                ],
                vec![paragraph(vec![text("two")])],
            ],
        }]
    );
}

#[test]
fn a_quote_holds_its_blocks() {
    assert_eq!(
        parse("> quoted\n>\n> - item"),
        [Block::Quote(vec![
            paragraph(vec![text("quoted")]),
            Block::List {
                start: None,
                items: vec![vec![paragraph(vec![text("item")])]],
            },
        ])]
    );
}
```

The others: `headings_keep_their_level` (`#` and `###`, with emphasis inside),
`a_tight_list_gives_each_item_a_paragraph`, `a_loose_list_keeps_every_paragraph_of_an_item`,
`an_ordered_list_keeps_its_start` (`3.` → `Some(3)`) and `a_rule_sits_between_paragraphs`.

`unhandled_blocks_keep_their_text` drops the heading, list, quote and rule, which are
handled now, and keeps the code block, table, link, image and inline HTML. It adds a code
block inside a list item and a table inside a quote, and walks the tree with a recursive
`collect_text`, so the "nothing vanishes" guarantee now covers text inside containers too.

Red, with the new `Block` variants and their rendering in place but `parse` unchanged: 7
failed, 243 passed, 3 ignored. The reworked `unhandled_blocks_keep_their_text` passed,
since Step 1 already flattened everything.

```
test chat::markdown::test::a_loose_list_keeps_every_paragraph_of_an_item ... FAILED
test chat::markdown::test::a_rule_sits_between_paragraphs ... FAILED
test chat::markdown::test::a_tight_list_gives_each_item_a_paragraph ... FAILED
test chat::markdown::test::a_quote_holds_its_blocks ... FAILED
test chat::markdown::test::headings_keep_their_level ... FAILED
test chat::markdown::test::an_ordered_list_keeps_its_start ... FAILED
test chat::markdown::test::lists_nest_inside_items ... FAILED
```

Green after: 250 passed, 3 ignored, and `cargo clippy --all-targets` was clean.

**Eyeball under `dx serve`:** ask for "a short outline with a heading, a nested bulleted
list, a numbered list starting at 3, a quote and a horizontal rule". The heading is a
little larger and bold but not huge. Bullets and numbers sit indented inside the bubble,
the nested list uses hollow bullets, the numbers start at 3. The quote has a faint bar on
its left. The rule is a thin line. There are no blank lines between list items, and the
first and last block sit flush with the bubble's padding.

### Minimal implementation

- **`src/chat/markdown.rs`** —
  - `Block` gains `Heading { level: u8, inlines }`, `List { start: Option<u64>, items:
    Vec<Vec<Block>> }`, `Quote(Vec<Block>)` and `Rule`.
  - A private `Container` enum, `Quote(Vec<Block>)` or `List { start, items }`, with
    `into_block`. `Builder` gains `containers: Vec<Container>`.
  - `push_block` writes into the top container (a list's last item), or into `blocks`
    when none is open. `end_block` now goes through it, so a paragraph lands wherever it
    belongs.
  - `take_inlines` is the old first half of `end_block`: close every open span, take the
    inlines. `end_block` and the new `end_heading` share it.
  - New arms: `End(Heading(level))` → `end_heading`. `Start(BlockQuote | List)` →
    `open_container`. `End(BlockQuote | List)` → `close_container`. `Start(Item)` →
    `start_item`, which pushes an empty item onto the open list. `Rule` → `rule`. All of
    them call `end_block` first. `End(Item)`, `Start(Heading)`, `Start/End(Paragraph)`,
    code blocks and table cells still fall to the `Start(_) | End(_)` arm, which ends the
    current paragraph.
- **`src/ui/markdown.rs`** — `Markdown` is now just `render_blocks(&parse(&text))`. The
  recursive `render_blocks` maps `Paragraph` → `p`, `List` → `ul` or `ol { start }` of
  `li`s that recurse, `Quote` → `blockquote` that recurses, `Rule` → `hr`.
  `render_heading` maps the level to `h1`–`h6`.
- **`src/ui/chat.css`** — `p`, `ul`, `ol` and `blockquote` share the `0.6em` vertical
  margin. Headings are `1.3em`/`1.18em`/`1.08em` for levels 1–3 and `1em` below that, with
  more space above than below. Lists get `1.4em` of indent and an explicit `list-style`.
  Paragraphs inside an item lose their margins, and nested lists sit `0.25em` under their
  item. The quote has a `3px` `--tint-border` bar and is slightly faded. The rule is a
  `1px` `--tint-divider` line. The first-child/last-child rule that kept paragraphs flush
  now covers every block element.

### Why it works

- **Where does this block go?** `push_block` answers that in one place, from the top of
  the container stack. `- one\n  - inner` arrives as `Start(List) Start(Item) Text(one)
  Start(List) Start(Item) Text(inner) End(Item) End(List) End(Item) End(List)`. The
  inner `Start(List)` ends `one` as a paragraph in the outer item, then becomes the top
  frame. Its `End(List)` pops it, and `push_block` puts the finished `List` into the outer
  list's last item, right after `one`.
- **Tight lists need no special case.** In a tight list `pulldown-cmark` sends an item's
  text with no `Paragraph` around it. That text gathers in `inlines` like any other, and
  `end_block` wraps it in a `Paragraph` when the next boundary arrives: the nested list's
  `Start`, the `End(Item)`, or anything else. A loose list sends real `Paragraph` events,
  which end up in the same shape.
- **A list's items live inside its frame.** Items aren't containers of their own: an item
  is the last `Vec<Block>` of the open list, and `Start(Item)` just pushes a new one. That
  keeps the stack to two kinds of frame.
- **`End` always meets its own frame.** `pulldown-cmark` closes every block it opened,
  even at the end of half-streamed text, so popping the top frame on `End(List)` or
  `End(BlockQuote)` is always right, the same reasoning that makes `spans` safe.
- **Headings own their inlines.** `End(Heading)` takes whatever inlines gathered since the
  heading started, so emphasis inside a heading works with no extra code.
- **No stray blank lines.** The bubble keeps `white-space: pre-wrap`, so a stray newline
  text node between two blocks would show as a blank line. RSX never creates whitespace
  between elements: the only `"\n"` text nodes are `SoftBreak`s, inside paragraphs. So
  pre-wrap needs no reset, and soft breaks still break lines.
- **Lists need an explicit `list-style`.** The bubble is itself an `li` inside the
  messages `ul`, which sets `list-style: none`. `list-style` is inherited, so without
  `list-style: disc` and `decimal` a reply's bullets and numbers would vanish.

### Forks taken

- **Tight and loose items have the same shape.** Item text always becomes a `Paragraph`.
  The alternative is `List { tight: bool, .. }` and rendering tight items' inlines bare in
  the `li`. In a narrow bubble the difference is a few pixels of spacing, and CSS
  (`li > p { margin: 0 }`) already makes a tight list look tight. The cost: a loose list
  looks tight too, apart from `0.4em` between paragraphs of the same item.
- **Heading levels map one to one to `h1`–`h6`.** CSS sizes them down to fit a bubble
  (`h1` is only `1.3em`). The alternative is to shift the levels (`#` → `h3`), which saves
  CSS but lies about the structure.
- **`level: u8`, not `HeadingLevel`.** The tree stays free of `pulldown-cmark` types, like
  the rest of it. `level as u8` works because `HeadingLevel` is `H1 = 1, H2, …`.
- **`start: Option<u64>` doubles as "ordered or not".** `None` is a bulleted list, `Some`
  is numbered. That's the shape `Tag::List` already uses, so no extra `ordered: bool`.
- **Headings render through a helper.** RSX can't pick a tag name at run time, so
  `render_heading` matches the level to six `rsx!` arms. Levels beyond 6 can't occur; the
  catch-all arm is `h6`.

### Scope note

- Code blocks are still paragraphs (Step 3); inside a list item or a quote they now land
  in that item or quote, still as paragraphs. Tables are Step 4, links, images and raw HTML
  Step 5.
- GFM task lists (`- [ ]`) stay off; `TaskListMarker` is still ignored.

### Review notes (from the `simplify` pass)

- **Applied:** dropped a `finish` method that popped any containers still open at the end
  of the text. `pulldown-cmark` closes every block itself, and `spans` already relies on
  the same guarantee.
- **Skipped:** a separate `Container::Item` frame. Treating an item as the list's last
  `Vec<Block>` is less code and needs no "is the parent a list?" check on `End(Item)`.
