use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Block {
    Paragraph(Vec<Inline>),
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

#[derive(Default)]
struct Builder {
    blocks: Vec<Block>,
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

    fn end_block(&mut self) {
        while !self.spans.is_empty() {
            self.close();
        }
        if !self.inlines.is_empty() {
            let inlines = std::mem::take(&mut self.inlines);
            self.blocks.push(Block::Paragraph(inlines));
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
            Event::Start(Tag::Link { .. } | Tag::Image { .. })
            | Event::End(TagEnd::Link | TagEnd::Image)
            | Event::TaskListMarker(_) => {}
            Event::Start(_) | Event::End(_) | Event::Rule => builder.end_block(),
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
    fn unhandled_blocks_keep_their_text() {
        let blocks = parse(
            "# Title\n\n- one\n- two\n\n> quoted\n\n```\nlet x = 1;\n```\n\n| a | b |\n|---|---|\n| c | d |\n\n[link](https://example.com) ![alt](x.png) <b>html</b>\n\n---",
        );

        let mut seen = String::new();
        for Block::Paragraph(inlines) in &blocks {
            for inline in inlines {
                if let Inline::Text(text) | Inline::Code(text) = inline {
                    seen.push_str(text);
                    seen.push('|');
                }
            }
        }

        for expected in [
            "Title",
            "one",
            "two",
            "quoted",
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
        ] {
            assert!(
                seen.contains(expected),
                "{expected:?} was dropped from {seen:?}"
            );
        }
    }
}
