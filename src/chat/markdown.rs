use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

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
    Rule,
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
            Event::Rule => builder.rule(),
            Event::Start(Tag::Link { .. } | Tag::Image { .. })
            | Event::End(TagEnd::Link | TagEnd::Image)
            | Event::TaskListMarker(_) => {}
            Event::Start(_) | Event::End(_) => builder.end_block(),
            Event::Code(code) => builder.push(Inline::Code(code.into_string())),
            Event::SoftBreak => builder.push(Inline::SoftBreak),
            Event::HardBreak => builder.push(Inline::HardBreak),
            Event::Text(text)
            | Event::Html(text)
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
    use super::{parse, Block, Inline};

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
                Block::Rule => {}
            }
        }
    }

    #[test]
    fn unhandled_blocks_keep_their_text() {
        let blocks = parse(
            "```\nlet x = 1;\n```\n\n| a | b |\n|---|---|\n| c | d |\n\n[link](https://example.com) ![alt](x.png) <b>html</b>\n\n- item\n\n  ```\n  nested code\n  ```\n\n> | e | f |\n> |---|---|",
        );

        let mut seen = String::new();
        collect_text(&blocks, &mut seen);

        for expected in [
            "let x = 1;",
            "a",
            "b",
            "c",
            "d",
            "link",
            "alt",
            "<b>",
            "html",
            "</b>",
            "item",
            "nested code",
            "e",
            "f",
        ] {
            assert!(
                seen.contains(expected),
                "{expected:?} was dropped from {seen:?}"
            );
        }
    }
}
