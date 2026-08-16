use rbook::Epub;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TocEntry {
    pub(crate) label: String,
    pub(crate) depth: usize,
    pub(crate) href: String,
}

pub(crate) fn toc_entries(epub: &Epub) -> Vec<TocEntry> {
    let Some(root) = epub.toc().contents() else {
        return Vec::new();
    };

    root.flatten()
        .filter_map(|entry| {
            Some(TocEntry {
                label: entry.label().to_string(),
                depth: entry.depth().saturating_sub(1),
                href: entry.href()?.decode().trim_start_matches('/').to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn the_toc_flattens_to_a_depth_tagged_list_in_reading_order() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");

        let entries = toc_entries(&epub);

        assert_eq!(entries.len(), 18);

        assert_eq!(entries[0].label, "The Adventures of Sherlock Holmes");
        assert_eq!(entries[0].depth, 0);
        assert_eq!(
            entries[0].href,
            "OEBPS/5186027266282590649_1661-h-0.htm.xhtml#pgepubid00000"
        );

        assert_eq!(entries[2].label, "I. A SCANDAL IN BOHEMIA");
        assert_eq!(entries[2].depth, 0);
        assert_eq!(entries[3].label, "I.");
        assert_eq!(entries[3].depth, 1);
        assert_eq!(entries[5].label, "III.");
        assert_eq!(entries[5].depth, 1);
        assert_eq!(entries[6].label, "II. THE RED-HEADED LEAGUE");
        assert_eq!(entries[6].depth, 0);

        assert_eq!(entries[17].label, "THE FULL PROJECT GUTENBERG™ LICENSE");
    }
}
