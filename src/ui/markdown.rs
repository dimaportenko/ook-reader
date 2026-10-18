use dioxus::prelude::*;

use crate::chat::markdown::{parse, Block, Inline};

#[component]
pub(crate) fn Markdown(text: String) -> Element {
    rsx! {
        for block in parse(&text) {
            match block {
                Block::Paragraph(inlines) => rsx! {
                    p { {render_inlines(&inlines)} }
                },
            }
        }
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
