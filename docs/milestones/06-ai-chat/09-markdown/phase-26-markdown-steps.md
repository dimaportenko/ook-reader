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
2. ~~**Headings, lists, quotes and rules.**~~ `Block::Heading`, `List { start: Option<u64>,
   items }` with nested lists and tight/loose items, `Quote(Vec<Block>)` and `Rule`. Add CSS
   for heading sizes inside a bubble, list indentation and the quote's border. **Done** —
   `fb766cc`.
3. ~~**Code blocks.**~~ `Block::Code { lang: Option<String>, text }` for both fenced and
   indented blocks, plus styling for inline `code`. Use a monospaced font, a theme-tinted
   background, and `overflow-x: auto` so long lines scroll. A test pins that an unclosed
   fence, as seen mid-stream, still yields a code block that holds the partial text.
   **Done** — `e718dca`.
4. ~~**Tables.**~~ `Block::Table { align, head, rows }` with each cell holding inlines, from
   `Options::ENABLE_TABLES`. It renders as a real `table` inside a wrapper that scrolls
   sideways. **Done** — `fd8da23`.
5. ~~**Links, images and raw HTML.**~~ `Inline::Link { href, children }` only when the URL's
   scheme is `http`, `https` or `mailto` (case-insensitive). Any other link keeps just its
   children. Images become their alt text. `Html` / `InlineHtml` become literal text. Tests
   cover `javascript:`, a relative href and a `<script>` tag. **Done** — `6c67a8a`.
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

> **Status:** done — committed in `fb766cc` (250 tests green, 3 ignored; clippy clean). The drawer rendering is still to be confirmed by hand under `dx serve`.

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

## Step 3 — Code blocks

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `e718dca` (257 tests green, 3 ignored; clippy clean). The drawer rendering is still to be confirmed by hand under `dx serve`.

**What it is.** A fenced block (```` ```rust ````) or an indented block in a reply now
shows as a code box: a monospaced font on a slightly darker tint, every space and line
break kept, and long lines scrolling sideways inside the bubble instead of wrapping or
widening the drawer. Inline `` `code` `` gets the same font and a small tinted pill. A
fence that hasn't closed yet, mid-stream, is already a code box holding what has arrived.

### The crux of this step: a mode, not a frame

Code is the first block whose text is **not** Markdown. `**not bold**` inside a fence has
to stay exactly that, and so does every space. `pulldown-cmark` already does the hard part:
inside a code block it sends only `Text` events, no `Strong` and no `SoftBreak`, with the
line breaks inside the text. So the builder doesn't need a third stack. It needs a
**mode**: while a code block is open, `Text` goes into a plain `String` instead of
becoming an `Inline`. A code block can't hold another block, so one `Option` is enough.

### Runnable check first

`src/chat/markdown.rs` — seven new tests, one helper `code(lang, text)`, and the walk-the-
tree test reworked:

```rust
#[test]
fn an_unclosed_fence_mid_stream_holds_the_partial_text() {
    assert_eq!(
        parse("```rust\nfn main() {"),
        [code(Some("rust"), "fn main() {")]
    );
}

#[test]
fn a_code_block_sits_inside_a_list_item() {
    assert_eq!(
        parse("- run:\n\n  ```sh\n  cargo test\n  ```\n- done"),
        [Block::List {
            start: None,
            items: vec![
                vec![
                    paragraph(vec![text("run:")]),
                    code(Some("sh"), "cargo test")
                ],
                vec![paragraph(vec![text("done")])],
            ],
        }]
    );
}

#[test]
fn code_text_is_kept_verbatim() {
    assert_eq!(
        parse("```\n**not bold** <b>\n\n  `spaced`\n```"),
        [code(None, "**not bold** <b>\n\n  `spaced`")]
    );
}
```

The others: `a_fenced_block_keeps_its_language_and_text` (a multi-line `rust` block with
indentation), `only_the_first_word_of_the_info_string_is_the_language` (`python
title="x.py"` → `python`), `an_empty_info_string_has_no_language` and
`an_indented_block_is_code` (a paragraph, then four-space-indented lines).

`unhandled_blocks_keep_their_text` drops its two code blocks, which are handled now, and
gains a table inside a list item in their place, so it still checks that a degraded block
inside a container keeps its text. `collect_text` learns `Block::Code`.

Red, with `Block::Code` and its rendering in place but `parse` unchanged: 7 failed, 250
passed, 3 ignored.

```
test chat::markdown::test::a_fenced_block_keeps_its_language_and_text ... FAILED
test chat::markdown::test::an_unclosed_fence_mid_stream_holds_the_partial_text ... FAILED
test chat::markdown::test::a_code_block_sits_inside_a_list_item ... FAILED
test chat::markdown::test::an_indented_block_is_code ... FAILED
test chat::markdown::test::code_text_is_kept_verbatim ... FAILED
test chat::markdown::test::an_empty_info_string_has_no_language ... FAILED
test chat::markdown::test::only_the_first_word_of_the_info_string_is_the_language ... FAILED
```

Green after: 257 passed, 3 ignored, and `cargo clippy --all-targets` was clean.

**Eyeball under `dx serve`:** ask for "a Rust function with one very long line, and a
sentence that mentions `cargo test` inline". While the fence is still open, the code box is
already there and grows line by line. Once done, the indentation is kept, the long line
scrolls sideways inside the box (the bubble and the drawer stay their width), and the box
sits flush with the bubble's padding when it is the first or last block. The inline
`cargo test` is monospaced on a small pill. Check light, sepia and dark themes: the tint
follows the text colour, so the box should be visible in all three.

### Minimal implementation

- **`src/chat/markdown.rs`** —
  - `Block` gains `Code { lang: Option<String>, text: String }`.
  - `Builder` gains `code: Option<(Option<String>, String)>`: `Some` while a code block is
    open, holding its language and the text so far.
  - `start_code(kind)` ends the current paragraph, takes the first whitespace-separated
    word of a fenced block's info string as the language (an empty info string gives
    `None`), and `Indented` gives `None`.
  - `text(&str)` is the new home of `Event::Text`: into the code buffer when one is open,
    otherwise `push(Inline::Text)` as before. `Html`, `InlineHtml`, math and footnote
    references still push `Inline::Text` directly; they never occur inside a code block.
  - `end_code` drops one trailing `\n`, if there is one, and `push_block`s the
    `Block::Code`, so it lands in the open list item or quote like any other block.
  - New arms: `Start(Tag::CodeBlock(kind))` → `start_code`, `End(TagEnd::CodeBlock)` →
    `end_code`. Still no catch-all over whole events.
- **`src/ui/markdown.rs`** — `Block::Code` renders as `pre { "data-lang": lang, code {
  "{text}" } }`. `lang.as_deref()` is an `Option<&str>`, and Dioxus leaves the attribute
  off when it's `None`.
- **`src/ui/chat.css`** —
  - `code` and `pre` share a system monospace stack and `0.9em`. Inline `code` gets a
    small padding, a rounded corner and the `--tint-fill` background.
  - `pre` gets padding, a rounded corner, the same `--tint-fill`, `overflow-x: auto`,
    `white-space: pre` and `overflow-wrap: normal`. `pre code` drops the pill's padding
    and background and keeps `1em`, so the font isn't shrunk twice.
  - `pre` joins the `0.6em` block margin and the first-child/last-child flush rules.
  - `.chat_panel__turn` gains `min-width: 0` next to its `max-width: 85%`.

### Why it works

- **The parser already separated code from Markdown.** Inside a code block `pulldown-cmark`
  sends `Start(CodeBlock)`, one or more `Text` events (typically one per line), then
  `End(CodeBlock)`. No inline events in between. So switching where
  `Text` goes is the whole change, and concatenating the pieces gives back the source
  exactly, leading spaces included: the parser has already removed the fence's own
  indentation (the two spaces of the list item in the test).
- **The trailing newline.** Every line of a closed block arrives with its `\n`, so the text
  always ends in one. Left in, `pre` would render an empty last line, a visibly taller box
  at the bottom. `end_code` drops exactly one, so a block that really ends in a blank line
  keeps it. The mid-stream line `fn main() {` comes without a newline, and nothing is
  removed.
- **Mid-stream is still free.** An unclosed fence runs to the end of the text, and
  `pulldown-cmark` closes it with `End(CodeBlock)` when the input runs out, the same
  guarantee `spans` and `containers` rely on. So the partial text becomes a code box on the
  very first render, and the next delta just adds to it.
- **Why the long line scrolls instead of widening.** `white-space: pre` stops the line
  from wrapping (it would inherit the bubble's `pre-wrap` otherwise), and `overflow-wrap:
  normal` stops the bubble's `anywhere` from breaking it. The line can only fit by
  scrolling, which `overflow-x: auto` allows. The `pre` is a block, so it's exactly as
  wide as the bubble. The bubble is a flex item in a column, so its width is its cross
  size: it shrinks to fit its content, up to `max-width: 85%`, and the messages list
  clips anything wider (`overflow-x: hidden`). `min-width: 0` says outright that a long
  line doesn't set a floor for the bubble's width.
- **The tint follows the theme.** `--tint-fill` is `currentColor` at 10% over whatever is
  behind, so the box is a shade darker on a light theme and lighter on a dark one, with no
  per-theme rule. On a user bubble, which already uses `--tint-fill`, it stacks and is still
  visible.

### Forks taken

- **A mode, not a frame on `containers`.** A `Container::Code` would make `push_block`
  answer a question that can't happen (a block inside code). An `Option` field says "code
  can't nest" in the type.
- **Drop one trailing newline.** The alternatives: keep it, and let CSS hide the empty
  line (it can't, reliably), or `trim_end` every trailing newline, which loses a block's
  intentional blank last line.
- **`data-lang` on `pre`, not `class: "language-rust"` on `code`.** No highlighter reads
  it yet, so the attribute only has to be visible and stylable. `language-*` is the
  convention highlighters like `highlight.js` look for, and is a one-line change if one
  arrives.
- **The language is kept as typed.** ```` ```Rust ```` stays `Rust`. Normalising to lower
  case would only matter once something reads it.
- **A system monospace stack.** `ui-monospace` first, then the platform faces, ending in
  `monospace`. A bare `monospace` would trigger browsers' small default monospace size.

### Scope note

- No syntax highlighting; the language is recorded and shown as an attribute only.
- No copy button. It would be the first interactive element in a reply.
- Tables are Step 4, links, images and raw HTML Step 5.

### Review notes (from the `simplify` pass)

- **Applied:** moved the font stack and size onto `:is(code, pre)` so `pre` and inline
  `code` share one rule, and set `pre code` back to `1em` so code in a block isn't
  `0.9 × 0.9`.
- **Skipped:** `strip_suffix('\n')` for the trailing newline. It returns a borrowed slice,
  which would mean a second allocation; `ends_with` then `pop` edits the `String` in place.

## Step 4 — Tables

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `fd8da23` (262 tests green, 3 ignored; clippy clean). The drawer rendering is still to be confirmed by hand under `dx serve`.

**What it is.** A GFM table in a reply now shows as a real table: a bold, tinted header
row, thin cell borders, and each column aligned the way its delimiter row says (`:--`,
`:-:`, `--:`). Bold and inline code work inside a cell. A table wider than the bubble
scrolls sideways inside it. Mid-stream, the table appears as soon as the delimiter row is
complete, header first, and grows a row at a time.

### The crux of this step: the head is just the first row

`pulldown-cmark`'s table events (checked in the 0.13.4 source and by printing them) are:

```
Start(Table([Left, Right, ...]))
  Start(TableHead)                    <- holds cells directly, no TableRow inside
    Start(TableCell) ...inlines... End(TableCell)
  End(TableHead)
  Start(TableRow)                     <- one per body row; there is no TableBody tag
    Start(TableCell) ...inlines... End(TableCell)
  End(TableRow)
End(Table)
```

`TableHead` and `TableRow` both mean "a row starts", and the head always comes first. So
the builder treats them the same: each pushes an empty row, each `End(TableCell)` takes the
current inlines as one cell, and `End(Table)` splits the first row off as the head. Like a
code block, a table can't hold a block or another table, so it's a second **mode**
(`Option`), not a frame on `containers`. Cells reuse the span stack untouched: a cell's
inlines are collected exactly as a paragraph's would be.

Two shapes come straight from the parser and needed no code, only tests:

- **A header plus a delimiter row and no body yet** is already a table: `Table`,
  `TableHead`, `End(Table)`. It becomes a table with `rows: []`.
- **A short row is padded** by `pulldown-cmark` with empty cells up to the head's width,
  and extra cells are dropped. So every row is as wide as `align`.
- **A half-typed delimiter row** (`|--`) is not a table yet, just a paragraph with the
  pipes as text. On the next delta it becomes one.

### Runnable check first

`src/chat/markdown.rs` — five new tests and a helper `table(align, head, rows)`:

```rust
#[test]
fn a_table_with_no_body_rows_yet_is_just_its_head() {
    assert_eq!(
        parse("| a | b |\n|---|---|"),
        [table(
            vec![Align::None, Align::None],
            vec![vec![text("a")], vec![text("b")]],
            vec![],
        )]
    );
}

#[test]
fn a_short_row_is_padded_to_the_head() {
    assert_eq!(
        parse("| a | b | c |\n|---|---|---|\n| x |"),
        [table(
            vec![Align::None; 3],
            vec![vec![text("a")], vec![text("b")], vec![text("c")]],
            vec![vec![vec![text("x")], vec![], vec![]]],
        )]
    );
}
```

The others: `a_table_keeps_its_head_rows_and_alignment` (left, right, center and unset
columns; `**tea**` and `` `2` `` in body cells), `a_table_sits_inside_a_list_item` and
`a_half_typed_delimiter_row_is_still_a_paragraph`.

`unhandled_blocks_keep_their_text` loses its tables, which are handled now. It keeps the
link, the image and the raw HTML at the top level, and gains them inside a list item, a
quote and a table cell, so it still checks that what is left unhandled (links and images
until Step 5, raw HTML for good) keeps its text wherever it sits. `collect_text` learns
`Block::Table`.

Red, with `Block::Table`, `Align` and the rendering in place but `parse` unchanged: 4
failed, 258 passed, 3 ignored. The half-typed delimiter test and the reworked walk-the-tree
test passed already: they pin behaviour the parser had before this step.

```
test chat::markdown::test::a_short_row_is_padded_to_the_head ... FAILED
test chat::markdown::test::a_table_keeps_its_head_rows_and_alignment ... FAILED
test chat::markdown::test::a_table_sits_inside_a_list_item ... FAILED
test chat::markdown::test::a_table_with_no_body_rows_yet_is_just_its_head ... FAILED
```

Green after: 262 passed, 3 ignored, and `cargo clippy --all-targets` was clean.

**Eyeball under `dx serve`:** ask for "a markdown table comparing five programming
languages across six columns, with the numeric columns right-aligned and some bold
names". While it streams, the pipes show as text for a moment, then the header appears
as a table and rows are added one at a time. Once done, the numbers line up on the right,
the header is bold on a tint, and the table scrolls sideways inside the bubble without
widening the bubble or the drawer. Cells should not wrap one letter per line. Check light,
sepia and dark: the borders and header tint follow the text colour.

### Minimal implementation

- **`src/chat/markdown.rs`** —
  - `Block` gains `Table { align: Vec<Align>, head: Vec<Cell>, rows: Vec<Vec<Cell>> }`,
    with `type Cell = Vec<Inline>` (clippy flagged the nested `Vec`s as too complex).
  - Its own `Align { None, Left, Center, Right }`, with `From<pulldown_cmark::Alignment>`,
    so the tree doesn't expose the parser's types.
  - `Builder` gains `table: Option<(Vec<Align>, Vec<Vec<Cell>>)>`.
  - `start_table` ends the current paragraph and opens the mode; `start_row` pushes an
    empty row (for both `TableHead` and `TableRow`); `end_cell` moves `take_inlines()` into
    the last row; `end_table` takes the first row as the head and `push_block`s the table,
    so it lands in the open list item or quote.
  - New arms for `Start(Table)`, `Start(TableHead | TableRow)`, `End(TableCell)`,
    `End(Table)`, and an explicit no-op for `Start(TableCell)` and
    `End(TableHead | TableRow)`. Still no catch-all over whole events.
- **`src/ui/markdown.rs`** — `Block::Table` renders as `div.chat_panel__table > table`
  with a `thead` row of `th` and a `tbody` of `td` rows. Each cell gets `text_align:
  text_align(align, column)`, an `Option<&str>`: `None` for an unset column, so Dioxus
  leaves the style off.
- **`src/ui/chat.css`** —
  - `.chat_panel__table` gets `overflow-x: auto` and joins the `0.6em` block margin and the
    first-child/last-child flush rules.
  - The `table` gets `border-collapse: collapse`, `overflow-wrap: normal` and `white-space:
    normal`. Cells get padding, a `--tint-border` border and `vertical-align: top`; `th`
    gets `--tint-fill`, weight 600 and a left default alignment.

### Why it works

- **One code path for both kinds of row.** Because `TableHead` holds cells directly, not a
  `TableRow`, a builder that only listened for `TableRow` would put the head's cells
  nowhere. Treating "head" as "row 0" means there is no `in_head` flag to keep in sync,
  and a table that has only a head (mid-stream) falls out as `rows: []`.
- **Mid-stream is still free.** The parser closes the table at the end of the text, the
  same guarantee code blocks and containers rely on. Before the delimiter row is complete
  there is no table at all, just a paragraph, which is the honest rendering of what has
  arrived.
- **Why the table scrolls instead of squeezing.** The bubble sets `overflow-wrap:
  anywhere`, and `anywhere` (unlike `break-word`) lowers an element's min-content width
  to one character. An auto-layout table sizes columns from their min-content width, so it
  would rather shrink every column to a letter wide than overflow. `overflow-wrap: normal`
  on the table puts the real minimum back: words stay whole, the table gets wider than the
  bubble, and the wrapper's `overflow-x: auto` scrolls it. The wrapper is a block, so it's
  exactly as wide as the bubble, which `min-width: 0` and `max-width: 85%` already cap.
- **`white-space: pre-wrap` doesn't break the layout.** Dioxus creates no whitespace text
  nodes between elements, so there are no stray newlines between `tr` and `td` for
  `pre-wrap` to show. `white-space: normal` on the table is a guard: cell text from the
  parser is already trimmed, and nothing inside a cell needs to keep runs of spaces.
- **The alignment is inline, not a class.** An inline `text-align` beats the `th` rule's
  `left`, so an aligned header follows its column and an unset one stays left.

### Forks taken

- **A mode, not a container frame.** Same reasoning as code blocks: a table can't hold
  blocks, and an `Option` says so in the type.
- **Own `Align` enum with `None`**, mirroring `pulldown-cmark`'s, rather than
  `Option<Align>` with three variants. It keeps `align` the same length as the head and
  maps one to one; the cost is a variant named `None` next to `Option::None`.
- **Head as row 0, split at the end**, rather than a `head` field filled while an
  `in_head` flag is set.
- **Rendering trusts the parser's padding**: cells are enumerated per row and the
  alignment is looked up by index (`align.get(column)`), so a row of a different width
  would still render, just without alignment past the end.
- **Unset columns are left-aligned** in the header (GitHub's look) rather than the
  browser's centred `th`.

### Scope note

- No sorting, no sticky header, no per-cell wrapping control.
- Links and images inside a cell still flatten to their text; that's Step 5.

### Review notes (from the `simplify` pass)

- **Applied:** `type Cell = Vec<Inline>`, which names what the nested `Vec`s mean and
  clears clippy's `type_complexity`.
- **Skipped:** a shared `render_row(cells, align, th_or_td)` helper for the head and body
  rows. RSX can't pick the tag name at run time, so it would need a `match` on the tag
  inside, which is more code than the two short loops it replaces.

## Step 5 — Links, images and raw HTML

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `6c67a8a` (271 tests green, 3 ignored; clippy clean). The drawer rendering and link clicks are still to be confirmed by hand under `dx serve`.

**What it is.** A link in a reply is now a real, underlined, accent-coloured link when it
points at `http://`, `https://` or `mailto:`, and clicking it opens the system browser or
mail app while the reader stays put. Any other link (`javascript:`, `data:`, `file:`, a
relative path, a `#fragment`) shows only its text. Autolinks (`<https://…>`,
`<me@example.com>`) are links too. An image shows as its alt text, and an image inside a
safe link becomes a link of that alt text. Raw HTML, `<script>` included, stays literal
text, as it has since Step 1.

### The crux of this step: a link is a span that may refuse to wrap

A link's events look like any other inline span: `Start(Link { link_type, dest_url, .. })`,
its children, `End(Link)`. So a link is one more frame on the existing `spans` stack, and
nested bold or code inside it works without new code. The twist is that the decision about
the href is made at `Start`, before any children arrive, and an unsafe link still has to
consume its `End`. So the frame is always pushed, carrying `Span::Link(Option<String>)`:
`Some(href)` wraps the children in `Inline::Link`, `None` hands the children back to the
parent unwrapped. Text then merges as usual, so `a [x](notes.md) b` is one `Text("a x b")`.

**Why only these three schemes, checked against dioxus-desktop 0.7.9's source:**

- `src/webview.rs` (lines ~369–393) installs wry's navigation handler. A navigation to a
  URL that `starts_with("http://")`, `"https://"` or `"mailto:"` calls
  `webbrowser::open(&url)` and returns `false`, cancelling it. Anything else (bar the
  `dioxus://` index) falls to `navigation_handler … unwrap_or(true)`: it is **allowed**,
  and would navigate the WebView away from the reader.
- The interpreter (`dioxus-interpreter-js` 0.7.9, `native.ts` `handleClickNavigate`) also
  intercepts clicks: when a click is delivered to Dioxus and no handler called
  `prevent_default`, the closest `a` has its default prevented and its raw `href`
  attribute sent over IPC as `browser_open`, which `app.rs` passes straight to
  `webbrowser::open`, whatever the scheme. So either path ends at the system browser, and
  the parser's filter is what keeps an arbitrary scheme from reaching it.
- The `starts_with` checks are case-sensitive, so the parser matches the three prefixes
  case-insensitively and **rewrites the prefix in lower case** (`HTTPS://X` becomes
  `https://X`). The href then matches the handler's check whether or not the WebView
  normalises the URL first. `https:x` (no slashes) is refused for the same reason.
- `target="_blank"` is not set. dioxus-desktop installs no new-window handler, so a
  `_blank` link would go down wry's new-window path instead of the navigation handler
  that is known to cancel and open externally.
- An email autolink (`LinkType::Email`) carries a bare address in `dest_url`, so the
  parser prefixes `mailto:` itself.

### Runnable check first

`src/chat/markdown.rs` — nine new tests and a helper `link(href, children)`:

```rust
#[test]
fn other_links_keep_only_their_text() {
    assert_eq!(
        parse(
            "a [js](javascript:alert(1)) b [rel](notes.md) c [data](data:text/html,hi) d [frag](#top) e [file](file:///etc/passwd) f [bare](https:x)"
        ),
        [paragraph(vec![text("a js b rel c data d frag e file f bare")])]
    );
}

#[test]
fn an_image_inside_a_link_is_a_link_of_its_alt_text() {
    assert_eq!(
        parse("[![logo](l.png)](https://x.example)"),
        [paragraph(vec![link("https://x.example", vec![text("logo")])])]
    );
}
```

The others: `safe_schemes_become_links`, `the_scheme_is_matched_in_any_case_and_lowercased`
(`HTTPS://` and `< MailTo:…>`), `autolinks_are_links` (a URL and an email autolink),
`formatting_nests_inside_a_link`, `a_refused_link_keeps_its_formatting`,
`an_image_becomes_its_alt_text` and `raw_html_is_literal_text` (`<script>` as an HTML block
and an inline `<b>`). The HTML block's text keeps pulldown-cmark's trailing `\n`; the test
pins that as it is.

The walk-the-tree test is renamed `every_word_survives_wherever_it_sits`: links are handled
now, so nothing in its input is "unhandled" any more, but it still checks that the text of
links, images and raw HTML survives at the top level, in a list item, a quote and a table
cell. `collect_text` gains `collect_inlines`, which descends into bold, italic,
strikethrough and link children, so a word inside a link still counts.

Red, with `Inline::Link` and its render arm in place but `parse` unchanged: 6 failed, 265
passed, 3 ignored. The three refusal tests and the image-alone test passed already: links
and images flattened to text before this step, which is the behaviour they pin.

```
test chat::markdown::test::the_scheme_is_matched_in_any_case_and_lowercased ... FAILED
test chat::markdown::test::an_image_inside_a_link_is_a_link_of_its_alt_text ... FAILED
test chat::markdown::test::autolinks_are_links ... FAILED
test chat::markdown::test::safe_schemes_become_links ... FAILED
test chat::markdown::test::raw_html_is_literal_text ... FAILED
test chat::markdown::test::formatting_nests_inside_a_link ... FAILED
```

(`raw_html_is_literal_text` failed only on the trailing `\n`, which it now expects.)

Green after: 271 passed, 3 ignored, and `cargo clippy --all-targets` was clean.

**Eyeball under `dx serve`:** ask for "three useful links about Rust, one written as a bare
`<https://…>` autolink, plus an email link and a markdown image". The links show in the
accent colour, underlined. Clicking one opens the default browser (or mail app) and the
reader stays on its page. The image shows only its alt text, and nothing is fetched. Then
ask it to "reply with exactly: `[click](javascript:alert(1))` and `[doc](notes.md)` as
markdown, not in a code block": both show as plain words that do nothing when clicked.
Check light, sepia and dark.

### Minimal implementation

- **`src/chat/markdown.rs`** —
  - `Inline` gains `Link { href: String, children: Vec<Inline> }`.
  - `Span` gains `Link(Option<String>)` and loses `Copy`. `wrap` now returns
    `Vec<Inline>`: one wrapped inline, or for `Link(None)` the children themselves; `close`
    pushes each, so text merges with its neighbours.
  - `safe_href(link_type, dest)` trims the destination, prefixes `mailto:` for an email
    autolink, and otherwise accepts it only if it starts with one of `SAFE_PREFIXES`
    (ignoring ASCII case), returning it with the prefix lower-cased.
  - `Start(Link)` opens the span, `End(Link)` joins the other span ends in `close()`, and
    `Start(Image)` / `End(Image)` get their own explicit no-op arm, so the alt text flows
    into whatever is open. Still no catch-all over whole events.
- **`src/ui/markdown.rs`** — `Inline::Link` renders as `a { href, title: href, … }` with
  the children through `render_inlines`.
- **`src/ui/chat.css`** — `.chat_panel__turn a` uses `--focused-border-color` (the theme's
  accent blue), an underline offset a little below the text, and a thicker underline on
  hover.

### Why it works

- **Safety is still by construction.** The only way an `a` reaches the DOM is
  `Inline::Link`, and the only way to build one is `safe_href` returning `Some`. The
  renderer has nothing to check.
- **The frame is pushed even when the link is refused.** `End(Link)` pops exactly one
  frame. Pushing nothing for an unsafe link would make its `End` close the enclosing bold
  or the wrong link.
- **The image needs no frame.** Its alt text arrives as ordinary inline events between
  `Start(Image)` and `End(Image)`, so ignoring the two tags leaves the alt text in the open
  paragraph, cell or link. Nothing about an image (URL, title) is kept, so the app never
  fetches a URL a model chose.
- **Mid-stream is unchanged.** pulldown-cmark emits a link only once `](url)` is complete;
  before that the brackets are plain text, which is what the reply really says so far.

### Forks taken

- **`Span::Link(Option<String>)` with a `Vec`-returning `wrap`**, rather than a separate
  `Span::Unlinked` variant or a special case in `close`. The cost is a one-element `Vec`
  per closed span.
- **Prefix match on the exact strings the handler checks** (`http://`, not just `http:`),
  rather than parsing the scheme with `split_once(':')`. It mirrors dioxus's own check, so
  every href that becomes an `a` is one the handler is known to cancel.
- **`title` set to the href**, so hovering shows where a link goes, since a model writes
  the visible text. The link's own markdown title is ignored.
- **No `rel` and no `target`.** `noopener`/`noreferrer` mean nothing when the WebView
  never navigates, and `_blank` would take a different wry path (above).
- **The accent is `--focused-border-color`**, the same blue as focus rings, rather than a
  new `--link` variable.

### Scope note

- No GFM bare-URL autolinking (`https://x` without angle brackets stays text).
- Reference-style links (`[x][ref]`) arrive as the same `Tag::Link`, so they should work;
  no test pins them.
- On iOS, `webbrowser` 1.2.1 opens only `http(s)` URLs, so a `mailto:` link there is
  cancelled by the handler and then fails to open. Not fixed here.
- The HTML block's trailing newline is kept; Step 6 can decide whether to trim it.
- The click path is read from source, not run: confirm under `dx serve` that the reader
  stays put and the browser opens.

## Step 6 — Review and refactor

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

**What it is.** A `code-review` pass (high effort) over the whole phase, `ba18374..HEAD`,
then a decision on each item the earlier steps deferred here, then a `simplify` pass over
the phase's `src/` changes (Steps 4 and 5 had skipped it). Three real bugs were fixed, all
in the styling. The parser's behavior is unchanged.

### Runnable check first

The three bugs are all in CSS or RSX. The repo has no SSR or DOM test harness, so none of
them can be pinned by a `#[test]` and none was watched going red. Their check is under
`dx serve`, still to be confirmed by hand:

- A wide table has borders, padding, a tinted header and a gap above and below, and it
  scrolls sideways inside the bubble instead of being clipped.
- In a tight list whose item holds a nested list, the nested list sits 0.25em under the
  item text, not 0.6em. Two paragraphs in one loose item are 0.4em apart.
- A reply whose first delta is only whitespace (`"\n"`) shows `...`, not an empty pill.

The one new unit test, for reference-style links, was written after the code it covers.
It passed first time, so it was checked by breaking its expected href and watching it go
red, then putting it back:

```rust
#[test]
fn reference_links_resolve_through_their_definitions() {
    assert_eq!(
        parse("see [docs][1] and [Rust] or [bad][2]\n\n[1]: https://docs.example\n[rust]: <HTTP://rust.example>\n[2]: javascript:x"),
        [paragraph(vec![
            text("see "),
            link("https://docs.example", vec![text("docs")]),
            text(" and "),
            link("http://rust.example", vec![text("Rust")]),
            text(" or bad"),
        ])]
    );
}
```

### Minimal implementation

- **`src/ui/chat.rs`** — `Styles` is `pub(crate)`, the same as `settings::Styles`. The
  streaming bubble shows `...` while `text.trim().is_empty()`.
- **`src/ui/markdown.rs`** — the table wrapper's class is `{ChatStyles::chat_panel__table}`
  instead of the literal `"chat_panel__table"`. `ul` and `ol` share a `render_items` helper
  for their `li` loop.
- **`src/ui/chat.css`** — the block-margin rule uses `:where(…)` instead of `:is(…)`. The
  two 13-entry `:is(…):first-child` / `:last-child` lists become
  `.chat_panel__turn :where(*):first-child` and `:last-child`.
- **`src/chat/markdown.rs`** — `End(TagEnd::Paragraph)` gets its own arm ahead of the
  catch-all, so you can see where a paragraph ends. The reference-link test.

### Why it works

- **The table class.** `#[css_module]` renames every class in `chat.css` to
  `<name>-<hash>`, and `Styles::<name>` is the only way to get the renamed string. Step 4
  wrote the class as a plain literal, so not one `.chat_panel__table` rule ever matched.
  The table had no borders, no padding, no margin and no sideways scroll, and
  `.chat_panel__messages { overflow-x: hidden }` clipped its right-hand columns.
- **The specificity.** `:is()` takes the specificity of its most specific argument. With
  `.chat_panel__table` in the list, the margin rule scored two classes, `(0,2,0)`, and beat
  `li > p`, `li > p + p` and `li :is(ul, ol)` at `(0,1,2)`. Those three rules were dead.
  `:where()` scores zero, so the rule drops to `(0,1,0)` and the list rules win again. The
  edge rules are now `(0,2,0)`, which still beats the margin, heading and list rules, as the
  old `(0,3,0)` did.
- **`:where(*)` for the edges.** It hits every first or last child at any depth, as the
  old lists did. The extra elements it now reaches (`strong`, `li`, `tr`, `td`) have no
  vertical margin, so for them it changes nothing, and a new block type needs no edit.
- **`trim()`.** `parse("\n")` is empty, so `Markdown` rendered nothing inside a padded
  bubble.

### Forks taken

- **Exported `chat::Styles`** rather than writing `:global(.chat_panel__table)` in the CSS,
  or giving `Markdown` its own `markdown.css` module. It's the pattern `settings::Styles`
  already uses. The cost is that `ui::markdown` depends on `ui::chat`, which only matters if
  `Markdown` is ever used outside the drawer (see *Review notes*).
- **No test for the CSS fixes.** A test that `Styles::chat_panel__table != "chat_panel__table"`
  would test the macro, not this code.

### Scope note

- Hand checks still to be confirmed under `dx serve`: each syntax rendering in a real
  reply, sideways scroll of code blocks and tables, the light, dark and sepia themes, and a
  link opening the system browser on desktop and on iOS.

### Review notes

From `code-review` (high) over `ba18374..HEAD`:

- **Fixed:** the table class never matched (above).
- **Fixed:** the `:is()` specificity that killed three list-spacing rules (above).
- **Fixed:** a whitespace-only first delta showed an empty bubble. A lone `#` or `-`
  mid-stream still shows an empty heading or bullet for a token. That is what the reply
  says so far, the same as an unclosed fence.
- **Applied:** `render_items` for the duplicated `ul`/`ol` loop, and `:where(*)` for the
  duplicated first/last-child lists.
- **Dismissed:** "re-parsing the whole reply per delta is O(n²)". This is the phase's
  recorded decision. A reply is a few KB.
- **Skipped:** cloning each history message's text into `Markdown`'s prop on every render.
  Unchanged props stop the child re-rendering, so the cost is one copy per turn per delta.
  `Rc<str>` would mean changing `Message`, which is outside this phase.
- **Skipped:** turning an inline `<br>` in a table cell into a `HardBreak`. Raw HTML is
  literal text by design. It's a candidate if real replies show it often.
- **Skipped:** dropping or showing `data-lang`. Step 3 kept the fence language on purpose,
  and a test pins it. A visible label or highlighting can read it later.
- **Skipped:** collapsing `Align` and `text_align` into stored CSS strings (see below).

The deferred punch list:

- **Skipped: SoftBreak as a space with `white-space: normal`.** A single newline in a reply
  is usually meant: a poem's lines, a quoted passage, a "Label:" line. The drawer showed
  those breaks in the raw-text days, and pre-wrap keeps them. The cost is that `pre` and
  the table have to reset `white-space`, and both already do.
- **Skipped: `Option<Align>` instead of `Align::None`.** `Align` mirrors pulldown's
  `Alignment` one-to-one, so the `From` impl reads as a plain mapping. The renderer's
  `match` is already exhaustive and returns `Option`. `Option<Align>` would trade one
  variant for a `match` that can't use `From`.
- **Skipped: trimming the HTML block's trailing `\n`.** It's invisible: a final newline in
  a pre-wrap block draws no extra line. Trimming would need its own `End(HtmlBlock)` arm.
- **Applied: a reference-link test** (above). `[x][1]`, a shortcut `[Rust]` with a
  case-folded label, and a refused `javascript:` definition all go through `safe_href`.
- **Accepted and documented: `mailto:` on iOS.** `webbrowser` 1.2.1's iOS backend
  (`ios.rs`, `target.get_http_url()?`) opens only `http(s)` URLs. On iOS, the navigation
  handler cancels a `mailto:` tap and the open then fails, so nothing happens. Dropping
  `mailto` on iOS only would make `parse` depend on the target, and tests would differ per
  platform. An email link in a reply about a book is rare, and its text still shows.
- **Applied: the `simplify` pass** (below).
- **The `Builder`** was flagged by both the review and `simplify` as four parallel pieces
  of state: the `spans` and `containers` stacks and the `code` and `table` modes. See
  below.

From the `simplify` pass (reuse, simplification, efficiency, altitude) over the phase's
`src/` diff:

- **Clean (reuse):** `safe_href` is the only scheme allowlist in `src/`, and the CSS uses
  the existing `--tint-*` and `--focused-border-color` variables.
- **Applied:** the explicit `End(TagEnd::Paragraph)` arm.
- **Skipped:** a recursive-descent parser (`fn blocks(events)`, `fn inlines(events)`) that
  would replace both stacks and both modes with the call stack. It's a rewrite of the
  phase's subject. The explicit stack is lesson 1 of the crux, and Step 3 recorded "a mode,
  not a frame" on purpose. It's the shape to try if a fifth kind of state ever lands.
- **Skipped:** one `Vec<Frame>` stack for quote, list, table and code, and
  `containers: Vec<Block>` in place of `Container`. Each needs an unreachable arm wherever
  only one kind of frame is valid. That is the same "can't happen" that the current
  `if let` guards express without a panic.
- **Skipped:** turning those `if let` guards into `expect`. A malformed event order
  should never crash the chat drawer mid-stream.
- **Skipped:** flushing pending inlines once at the top of the loop instead of in each
  block method. It needs a list of "inline events" that would have to stay in sync with
  `parse`'s match, and it is easier to get wrong than the current calls.
- **Skipped:** `Span::wrap` returning one `Inline` instead of a `Vec`. That is Step 5's
  recorded fork, and the cost is one small allocation per span.
- **Skipped:** a `push_text(&str)` that saves the allocation when text merges into the
  previous `Text`, and building the lower-cased href without `format!`. Microseconds per
  reply.
- **Skipped:** moving the Markdown rules into their own `markdown.css` module with a root
  element and flex `gap`, and scoping `pre-wrap` to text elements. That restyles every
  block while the hand checks are still pending. It's the candidate if `Markdown` is ever
  used outside the drawer.
- **Skipped:** a shared `TurnBody` for the history loop and the streaming bubble. The two
  differ (the bubble has placeholders), and there is only one caller of each.
- **Skipped:** a `render_row` helper for `th`/`td`. They are different elements, and the
  `if` inside would cost more than the four repeated lines.
