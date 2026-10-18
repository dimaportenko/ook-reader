use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Block {
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    List {
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    Quote(Vec<Block>),
    Code {
        lang: Option<String>,
        text: String,
    },
    Table {
        align: Vec<Align>,
        head: Vec<Cell>,
        rows: Vec<Vec<Cell>>,
    },
    Rule,
}

pub(crate) type Cell = Vec<Inline>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    None,
    Left,
    Center,
    Right,
}

impl From<Alignment> for Align {
    fn from(alignment: Alignment) -> Self {
        match alignment {
            Alignment::None => Align::None,
            Alignment::Left => Align::Left,
            Alignment::Center => Align::Center,
            Alignment::Right => Align::Right,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Code(String),
    Strikethrough(Vec<Inline>),
    SoftBreak,
    HardBreak,
}

#[derive(Clone, Copy)]
enum Span {
    Strong,
    Emphasis,
    Strikethrough,
}

impl Span {
    fn wrap(self, children: Vec<Inline>) -> Inline {
        match self {
            Span::Strong => Inline::Strong(children),
            Span::Emphasis => Inline::Emphasis(children),
            Span::Strikethrough => Inline::Strikethrough(children),
        }
    }
}

enum Container {
    Quote(Vec<Block>),
    List {
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
}

impl Container {
    fn into_block(self) -> Block {
        match self {
            Container::Quote(blocks) => Block::Quote(blocks),
            Container::List { start, items } => Block::List { start, items },
        }
    }
}

#[derive(Default)]
struct Builder {
    blocks: Vec<Block>,
    containers: Vec<Container>,
    inlines: Vec<Inline>,
    spans: Vec<(Span, Vec<Inline>)>,
    code: Option<(Option<String>, String)>,
    table: Option<(Vec<Align>, Vec<Vec<Cell>>)>,
}

impl Builder {
    fn push(&mut self, inline: Inline) {
        let target = match self.spans.last_mut() {
            Some((_, children)) => children,
            None => &mut self.inlines,
        };
        match (target.last_mut(), inline) {
            (Some(Inline::Text(last)), Inline::Text(text)) => last.push_str(&text),
            (_, inline) => target.push(inline),
        }
    }

    fn open(&mut self, span: Span) {
        self.spans.push((span, Vec::new()));
    }

    fn close(&mut self) {
        if let Some((span, children)) = self.spans.pop() {
            self.push(span.wrap(children));
        }
    }

    fn take_inlines(&mut self) -> Vec<Inline> {
        while !self.spans.is_empty() {
            self.close();
        }
        std::mem::take(&mut self.inlines)
    }

    fn push_block(&mut self, block: Block) {
        match self.containers.last_mut() {
            Some(Container::Quote(blocks)) => blocks.push(block),
            Some(Container::List { items, .. }) => match items.last_mut() {
                Some(item) => item.push(block),
                None => items.push(vec![block]),
            },
            None => self.blocks.push(block),
        }
    }

    fn end_block(&mut self) {
        let inlines = self.take_inlines();
        if !inlines.is_empty() {
            self.push_block(Block::Paragraph(inlines));
        }
    }

    fn end_heading(&mut self, level: HeadingLevel) {
        let inlines = self.take_inlines();
        self.push_block(Block::Heading {
            level: level as u8,
            inlines,
        });
    }

    fn text(&mut self, text: &str) {
        match &mut self.code {
            Some((_, code)) => code.push_str(text),
            None => self.push(Inline::Text(text.to_owned())),
        }
    }

    fn start_code(&mut self, kind: CodeBlockKind) {
        self.end_block();
        let lang = match kind {
            CodeBlockKind::Fenced(info) => info.split_whitespace().next().map(str::to_owned),
            CodeBlockKind::Indented => None,
        };
        self.code = Some((lang, String::new()));
    }

    fn end_code(&mut self) {
        if let Some((lang, mut text)) = self.code.take() {
            if text.ends_with('\n') {
                text.pop();
            }
            self.push_block(Block::Code { lang, text });
        }
    }

    fn start_table(&mut self, alignments: Vec<Alignment>) {
        self.end_block();
        let align = alignments.into_iter().map(Align::from).collect();
        self.table = Some((align, Vec::new()));
    }

    fn start_row(&mut self) {
        if let Some((_, rows)) = &mut self.table {
            rows.push(Vec::new());
        }
    }

    fn end_cell(&mut self) {
        let cell = self.take_inlines();
        if let Some(row) = self.table.as_mut().and_then(|(_, rows)| rows.last_mut()) {
            row.push(cell);
        }
    }

    fn end_table(&mut self) {
        if let Some((align, rows)) = self.table.take() {
            let mut rows = rows.into_iter();
            let head = rows.next().unwrap_or_default();
            self.push_block(Block::Table {
                align,
                head,
                rows: rows.collect(),
            });
        }
    }

    fn rule(&mut self) {
        self.end_block();
        self.push_block(Block::Rule);
    }

    fn open_container(&mut self, container: Container) {
        self.end_block();
        self.containers.push(container);
    }

    fn start_item(&mut self) {
        self.end_block();
        if let Some(Container::List { items, .. }) = self.containers.last_mut() {
            items.push(Vec::new());
        }
    }

    fn close_container(&mut self) {
        self.end_block();
        if let Some(container) = self.containers.pop() {
            self.push_block(container.into_block());
        }
    }
}

pub(crate) fn parse(text: &str) -> Vec<Block> {
    let mut builder = Builder::default();

    for event in Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES) {
        match event {
            Event::Start(Tag::Strong) => builder.open(Span::Strong),
            Event::Start(Tag::Emphasis) => builder.open(Span::Emphasis),
            Event::Start(Tag::Strikethrough) => builder.open(Span::Strikethrough),
            Event::End(TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough) => {
                builder.close()
            }
            Event::End(TagEnd::Heading(level)) => builder.end_heading(level),
            Event::Start(Tag::BlockQuote(_)) => {
                builder.open_container(Container::Quote(Vec::new()))
            }
            Event::Start(Tag::List(start)) => builder.open_container(Container::List {
                start,
                items: Vec::new(),
            }),
            Event::End(TagEnd::BlockQuote(_) | TagEnd::List(_)) => builder.close_container(),
            Event::Start(Tag::Item) => builder.start_item(),
            Event::Start(Tag::CodeBlock(kind)) => builder.start_code(kind),
            Event::End(TagEnd::CodeBlock) => builder.end_code(),
            Event::Start(Tag::Table(alignments)) => builder.start_table(alignments),
            Event::Start(Tag::TableHead | Tag::TableRow) => builder.start_row(),
            Event::End(TagEnd::TableCell) => builder.end_cell(),
            Event::End(TagEnd::Table) => builder.end_table(),
            Event::Start(Tag::TableCell) | Event::End(TagEnd::TableHead | TagEnd::TableRow) => {}
            Event::Rule => builder.rule(),
            Event::Start(Tag::Link { .. } | Tag::Image { .. })
            | Event::End(TagEnd::Link | TagEnd::Image)
            | Event::TaskListMarker(_) => {}
            Event::Start(_) | Event::End(_) => builder.end_block(),
            Event::Code(code) => builder.push(Inline::Code(code.into_string())),
            Event::SoftBreak => builder.push(Inline::SoftBreak),
            Event::HardBreak => builder.push(Inline::HardBreak),
            Event::Text(text) => builder.text(&text),
            Event::Html(text)
            | Event::InlineHtml(text)
            | Event::InlineMath(text)
            | Event::DisplayMath(text)
            | Event::FootnoteReference(text) => builder.push(Inline::Text(text.into_string())),
        }
    }

    builder.end_block();
    builder.blocks
}

#[cfg(test)]
mod test {
    use super::{parse, Align, Block, Inline};

    fn text(text: &str) -> Inline {
        Inline::Text(text.to_owned())
    }

    fn paragraph(inlines: Vec<Inline>) -> Block {
        Block::Paragraph(inlines)
    }

    #[test]
    fn plain_text_is_one_paragraph() {
        assert_eq!(
            parse("Hello there."),
            [paragraph(vec![text("Hello there.")])]
        );
    }

    #[test]
    fn a_blank_line_starts_a_new_paragraph() {
        assert_eq!(
            parse("One.\n\nTwo."),
            [paragraph(vec![text("One.")]), paragraph(vec![text("Two.")])]
        );
    }

    #[test]
    fn empty_text_has_no_blocks() {
        assert_eq!(parse(""), []);
    }

    #[test]
    fn inline_styles_wrap_their_text() {
        assert_eq!(
            parse("a **bold** b *it* c `x + 1` d ~~gone~~"),
            [paragraph(vec![
                text("a "),
                Inline::Strong(vec![text("bold")]),
                text(" b "),
                Inline::Emphasis(vec![text("it")]),
                text(" c "),
                Inline::Code("x + 1".to_owned()),
                text(" d "),
                Inline::Strikethrough(vec![text("gone")]),
            ])]
        );
    }

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

    #[test]
    fn headings_keep_their_level() {
        assert_eq!(
            parse("# Title\n\n### Sub *part*"),
            [
                Block::Heading {
                    level: 1,
                    inlines: vec![text("Title")],
                },
                Block::Heading {
                    level: 3,
                    inlines: vec![text("Sub "), Inline::Emphasis(vec![text("part")])],
                },
            ]
        );
    }

    #[test]
    fn a_tight_list_gives_each_item_a_paragraph() {
        assert_eq!(
            parse("- one\n- **two**"),
            [Block::List {
                start: None,
                items: vec![
                    vec![paragraph(vec![text("one")])],
                    vec![paragraph(vec![Inline::Strong(vec![text("two")])])],
                ],
            }]
        );
    }

    #[test]
    fn a_loose_list_keeps_every_paragraph_of_an_item() {
        assert_eq!(
            parse("1. one\n\n2. two\n\n   more"),
            [Block::List {
                start: Some(1),
                items: vec![
                    vec![paragraph(vec![text("one")])],
                    vec![paragraph(vec![text("two")]), paragraph(vec![text("more")])],
                ],
            }]
        );
    }

    #[test]
    fn an_ordered_list_keeps_its_start() {
        assert_eq!(
            parse("3. three\n4. four"),
            [Block::List {
                start: Some(3),
                items: vec![
                    vec![paragraph(vec![text("three")])],
                    vec![paragraph(vec![text("four")])],
                ],
            }]
        );
    }

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

    #[test]
    fn a_rule_sits_between_paragraphs() {
        assert_eq!(
            parse("above\n\n---\n\nbelow"),
            [
                paragraph(vec![text("above")]),
                Block::Rule,
                paragraph(vec![text("below")]),
            ]
        );
    }

    fn code(lang: Option<&str>, text: &str) -> Block {
        Block::Code {
            lang: lang.map(str::to_owned),
            text: text.to_owned(),
        }
    }

    #[test]
    fn a_fenced_block_keeps_its_language_and_text() {
        assert_eq!(
            parse("```rust\nfn main() {\n    println!(\"hi\");\n}\n```"),
            [code(Some("rust"), "fn main() {\n    println!(\"hi\");\n}")]
        );
    }

    #[test]
    fn only_the_first_word_of_the_info_string_is_the_language() {
        assert_eq!(
            parse("```python title=\"x.py\"\nx = 1\n```"),
            [code(Some("python"), "x = 1")]
        );
    }

    #[test]
    fn an_empty_info_string_has_no_language() {
        assert_eq!(parse("```\nplain\n```"), [code(None, "plain")]);
    }

    #[test]
    fn an_indented_block_is_code() {
        assert_eq!(
            parse("Run:\n\n    let x = 1;\n    let y = 2;\n"),
            [
                paragraph(vec![text("Run:")]),
                code(None, "let x = 1;\nlet y = 2;"),
            ]
        );
    }

    #[test]
    fn code_text_is_kept_verbatim() {
        assert_eq!(
            parse("```\n**not bold** <b>\n\n  `spaced`\n```"),
            [code(None, "**not bold** <b>\n\n  `spaced`")]
        );
    }

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

    fn table(align: Vec<Align>, head: Vec<Vec<Inline>>, rows: Vec<Vec<Vec<Inline>>>) -> Block {
        Block::Table { align, head, rows }
    }

    #[test]
    fn a_table_keeps_its_head_rows_and_alignment() {
        assert_eq!(
            parse("| Name | Qty | Note | Tag |\n|:--|--:|:-:|---|\n| **tea** | `2` | hot | x |"),
            [table(
                vec![Align::Left, Align::Right, Align::Center, Align::None],
                vec![
                    vec![text("Name")],
                    vec![text("Qty")],
                    vec![text("Note")],
                    vec![text("Tag")],
                ],
                vec![vec![
                    vec![Inline::Strong(vec![text("tea")])],
                    vec![Inline::Code("2".to_owned())],
                    vec![text("hot")],
                    vec![text("x")],
                ]],
            )]
        );
    }

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
    fn a_half_typed_delimiter_row_is_still_a_paragraph() {
        assert_eq!(
            parse("| a | b |\n|--"),
            [paragraph(vec![
                text("| a | b |"),
                Inline::SoftBreak,
                text("|--"),
            ])]
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

    #[test]
    fn a_table_sits_inside_a_list_item() {
        assert_eq!(
            parse("- totals:\n\n  | k | v |\n  |---|---|\n  | n | 1 |\n- done"),
            [Block::List {
                start: None,
                items: vec![
                    vec![
                        paragraph(vec![text("totals:")]),
                        table(
                            vec![Align::None, Align::None],
                            vec![vec![text("k")], vec![text("v")]],
                            vec![vec![vec![text("n")], vec![text("1")]]],
                        ),
                    ],
                    vec![paragraph(vec![text("done")])],
                ],
            }]
        );
    }

    fn collect_text(blocks: &[Block], seen: &mut String) {
        for block in blocks {
            match block {
                Block::Paragraph(inlines) | Block::Heading { inlines, .. } => {
                    for inline in inlines {
                        if let Inline::Text(text) | Inline::Code(text) = inline {
                            seen.push_str(text);
                            seen.push('|');
                        }
                    }
                }
                Block::List { items, .. } => {
                    for item in items {
                        collect_text(item, seen);
                    }
                }
                Block::Quote(blocks) => collect_text(blocks, seen),
                Block::Code { text, .. } => {
                    seen.push_str(text);
                    seen.push('|');
                }
                Block::Table { head, rows, .. } => {
                    for cell in head.iter().chain(rows.iter().flatten()) {
                        collect_text(&[paragraph(cell.clone())], seen);
                    }
                }
                Block::Rule => {}
            }
        }
    }

    #[test]
    fn unhandled_blocks_keep_their_text() {
        let blocks = parse(
            "[link](https://example.com) ![alt](x.png) <b>html</b>\n\n- [g](https://g.example) ![h](h.png)\n\n> <i>e</i> [f](f.md)\n\n| [c](c.md) | ![d](d.png) |\n|---|---|",
        );

        let mut seen = String::new();
        collect_text(&blocks, &mut seen);

        for expected in [
            "link", "alt", "<b>", "html", "</b>", "g", "h", "<i>", "e", "</i>", "f", "c", "d",
        ] {
            assert!(
                seen.contains(expected),
                "{expected:?} was dropped from {seen:?}"
            );
        }
    }
}
