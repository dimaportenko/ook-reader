use std::borrow::Cow;

pub(crate) const MAX_PASSAGE_CHARS: usize = 4_000;

pub(crate) const DEFAULT_PROMPT_TEMPLATE: &str = "Я читаю *{title}*
Автор: {author}
Розділ: {chapter}

Переклади та поясни наступний текст українською загалом та окремі слова або словосполучення, сталі вирази тощо

{selection}
";

pub(crate) struct Passage<'a> {
    pub(crate) title: &'a str,
    pub(crate) author: Option<&'a str>,
    pub(crate) chapter: Option<&'a str>,
    pub(crate) text: &'a str,
}

pub(crate) fn render(template: &str, passage: &Passage) -> String {
    let mut out = String::with_capacity(template.len() + passage.text.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let value = after
            .find('}')
            .and_then(|close| placeholder(&after[..close], passage).map(|v| (close, v)));
        match value {
            Some((close, value)) => {
                out.push_str(&value);
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    if !template.contains("{selection}") {
        out.truncate(out.trim_end_matches('\n').len());
        out.push_str("\n\n");
        out.push_str(&quoted(passage.text));
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn placeholder<'a>(name: &str, passage: &Passage<'a>) -> Option<Cow<'a, str>> {
    match name {
        "title" => Some(Cow::Borrowed(passage.title)),
        "author" => Some(Cow::Borrowed(passage.author.unwrap_or_default())),
        "chapter" => Some(Cow::Borrowed(passage.chapter.unwrap_or_default())),
        "selection" => Some(Cow::Owned(quoted(passage.text))),
        _ => None,
    }
}

fn quoted(text: &str) -> String {
    clipped(text)
        .lines()
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn clipped(text: &str) -> Cow<'_, str> {
    match text.char_indices().nth(MAX_PASSAGE_CHARS) {
        None => Cow::Borrowed(text),
        Some((end, _)) => Cow::Owned(format!("{}...", &text[..end])),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn passage(text: &str) -> Passage<'_> {
        Passage {
            title: "The Colour of Magic",
            author: Some("Terry Pratchett"),
            chapter: Some("The Colour of Magic"),
            text,
        }
    }

    #[test]
    fn each_placeholder_is_replaced_with_its_value() {
        let message = render(
            "T={title} A={author} C={chapter}\n{selection}",
            &passage("Rincewind ran."),
        );

        assert_eq!(
            message,
            "T=The Colour of Magic A=Terry Pratchett C=The Colour of Magic\n> Rincewind ran.\n"
        );
    }

    #[test]
    fn unknown_placeholders_are_left_exactly_as_typed() {
        let message = render("{titel} {} {Title} {selection", &passage("x"));

        assert_eq!(message, "{titel} {} {Title} {selection\n\n> x\n");
    }

    #[test]
    fn a_template_without_selection_gets_the_quote_after_a_blank_line() {
        let message = render("Explain this:\n", &passage("First.\nSecond."));

        assert_eq!(message, "Explain this:\n\n> First.\n> Second.\n");
    }

    #[test]
    fn the_default_template_names_the_book_and_quotes_the_passage() {
        let message = render(DEFAULT_PROMPT_TEMPLATE, &passage("Rincewind ran."));

        assert!(
            message.starts_with(
                "Я читаю *The Colour of Magic*\nАвтор: Terry Pratchett\nРозділ: The Colour of Magic\n\n"
            ),
            "{message}"
        );
        assert!(message.contains("українською"), "{message}");
        assert!(message.ends_with("\n\n> Rincewind ran.\n"), "{message:?}");
    }

    #[test]
    fn every_line_of_a_multi_line_passage_is_quoted() {
        let message = render("{selection}", &passage("First.\nSecond.\n\nThird."));

        assert_eq!(message, "> First.\n> Second.\n> \n> Third.\n");
    }

    #[test]
    fn missing_author_and_chapter_render_as_nothing() {
        let message = render(
            DEFAULT_PROMPT_TEMPLATE,
            &Passage {
                title: "Anonymous",
                author: None,
                chapter: None,
                text: "x",
            },
        );

        assert!(
            message.starts_with("Я читаю *Anonymous*\nАвтор: \nРозділ: \n\n"),
            "{message}"
        );
        assert!(!message.contains('{'), "{message}");
    }

    #[test]
    fn the_message_ends_on_a_fresh_line() {
        assert_eq!(render("{selection} — why?", &passage("x")), "> x — why?\n");
    }

    #[test]
    fn a_long_passage_is_cut_on_a_char_boundary() {
        let long = "é".repeat(MAX_PASSAGE_CHARS + 10);

        let message = render("{selection}", &passage(&long));

        assert_eq!(message, format!("> {}...\n", "é".repeat(MAX_PASSAGE_CHARS)));
    }
}
