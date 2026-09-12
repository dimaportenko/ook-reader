use std::borrow::Cow;

pub(crate) const MAX_MESSAGE_CHARS: usize = 4_000;

pub(crate) struct Passage<'a> {
    pub(crate) title: &'a str,
    pub(crate) author: Option<&'a str>,
    pub(crate) chapter: Option<&'a str>,
    pub(crate) text: &'a str,
}

pub(crate) fn draft(passage: &Passage) -> String {
    let mut out = format!("I'm reading *{}*", passage.title);
    if let Some(author) = passage.author {
        out.push_str(&format!(" by {author}"));
    }
    if let Some(chapter) = passage.chapter {
        out.push_str(&format!(", chapter \"{chapter}\""));
    }

    out.push_str(".\n\n");
    for line in clipped(passage.text).lines() {
        out.push_str(&format!("> {line}\n"));
    }
    out.push('\n');
    out
}

fn clipped(text: &str) -> Cow<'_, str> {
    match text.char_indices().nth(MAX_MESSAGE_CHARS) {
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
    fn the_draft_names_the_book_and_quotes_the_passage() {
        let draft = draft(&passage("Rincewind ran."));

        assert!(draft.contains("The Colour of Magic"), "{draft}");
        assert!(draft.contains("Terry Pratchett"), "{draft}");
        assert!(draft.contains("> Rincewind ran."), "{draft}");
        assert!(draft.ends_with("\n\n"), "the cursor lands on a fresh line: {draft:?}");
    }

    #[test]
    fn every_line_of_a_multi_line_passage_is_quoted() {
        let draft = draft(&passage("First.\nSecond."));

        assert!(draft.contains("> First.\n> Second.\n"), "{draft}");
    }

    #[test]
    fn missing_author_and_chapter_leave_no_holes() {
        let draft = draft(&Passage {
            title: "Anonymous",
            author: None,
            chapter: None,
            text: "x",
        });

        assert!(!draft.contains("by "), "{draft}");
        assert!(!draft.contains("chapter"), "{draft}");
    }

    #[test]
    fn a_long_passage_is_cut_on_a_char_boundary() {
        let long = "é".repeat(MAX_MESSAGE_CHARS + 10);

        let draft = draft(&passage(&long));

        let quoted = draft.split("> ").nth(1).unwrap();
        assert_eq!(quoted.chars().filter(|c| *c == 'é').count(), MAX_MESSAGE_CHARS);
        assert!(quoted.contains("..."), "truncation is visible: {quoted}");
    }
}
