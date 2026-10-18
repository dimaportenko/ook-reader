use dioxus::prelude::*;

use crate::chat::markdown::{parse, Block, Inline};

#[component]
pub(crate) fn Markdown(text: String) -> Element {
    render_blocks(&parse(&text))
}

fn render_blocks(blocks: &[Block]) -> Element {
    rsx! {
        for block in blocks {
            match block {
                Block::Paragraph(inlines) => rsx! {
                    p { {render_inlines(inlines)} }
                },
                Block::Heading { level, inlines } => render_heading(*level, inlines),
                Block::List { start: None, items } => rsx! {
                    ul {
                        for item in items {
                            li { {render_blocks(item)} }
                        }
                    }
                },
                Block::List { start: Some(start), items } => rsx! {
                    ol { start: "{start}",
                        for item in items {
                            li { {render_blocks(item)} }
                        }
                    }
                },
                Block::Quote(blocks) => rsx! {
                    blockquote { {render_blocks(blocks)} }
                },
                Block::Rule => rsx! { hr {} },
            }
        }
    }
}

fn render_heading(level: u8, inlines: &[Inline]) -> Element {
    let children = render_inlines(inlines);
    match level {
        1 => rsx! { h1 { {children} } },
        2 => rsx! { h2 { {children} } },
        3 => rsx! { h3 { {children} } },
        4 => rsx! { h4 { {children} } },
        5 => rsx! { h5 { {children} } },
        _ => rsx! { h6 { {children} } },
    }
}

fn render_inlines(inlines: &[Inline]) -> Element {
    rsx! {
        for inline in inlines {
            match inline {
                Inline::Text(text) => rsx! { "{text}" },
                Inline::Strong(children) => rsx! { strong { {render_inlines(children)} } },
                Inline::Emphasis(children) => rsx! { em { {render_inlines(children)} } },
                Inline::Code(code) => rsx! { code { "{code}" } },
                Inline::Strikethrough(children) => rsx! { del { {render_inlines(children)} } },
                Inline::SoftBreak => rsx! { "\n" },
                Inline::HardBreak => rsx! { br {} },
            }
        }
    }
}
