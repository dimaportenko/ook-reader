use rbook::Epub;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TocEntry {
    pub(crate) label: String,
    pub(crate) depth: usize,
    pub(crate) spine_index: usize,
    pub(crate) fragment: Option<String>,
}

pub(crate) fn toc_entries(epub: &Epub, docs: &[String]) -> Vec<TocEntry> {
    let Some(root) = epub.toc().contents() else {
        return Vec::new();
    };

    root.flatten()
        .filter_map(|entry| {
            let href = entry.href()?;
            let path = href.path().decode();
            let path = path.trim_start_matches('/');

            Some(TocEntry {
                label: entry.label().to_string(),
                depth: entry.depth().saturating_sub(1),
                spine_index: docs.iter().position(|doc| doc == path)?,
                fragment: href.fragment().map(|fragment| {
                    percent_encoding::percent_decode_str(fragment)
                        .decode_utf8_lossy()
                        .into_owned()
                }),
            })
        })
        .collect()
}

pub(crate) fn entry_index_for_spine(entries: &[TocEntry], spine_index: usize) -> Option<usize> {
    entries
        .iter()
        .position(|entry| entry.spine_index == spine_index)
        .or_else(|| {
            entries
                .iter()
                .rposition(|entry| entry.spine_index < spine_index)
        })
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use super::*;
    use crate::epub;

    #[test]
    fn the_toc_flattens_to_a_depth_tagged_list_in_reading_order() {
        let (epub, docs) =
            epub::open_with_spine(Path::new(crate::TEST_BOOK)).expect("open fixture book");

        let entries = toc_entries(&epub, &docs);

        assert_eq!(entries.len(), 18);

        assert_eq!(entries[0].label, "The Adventures of Sherlock Holmes");
        assert_eq!(entries[0].depth, 0);

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

    #[test]
    fn many_entries_can_name_one_spine_item() {
        let (epub, docs) =
            epub::open_with_spine(Path::new(crate::TEST_BOOK)).expect("open fixture book");

        let entries = toc_entries(&epub, &docs);

        assert_eq!(docs.len(), 15);
        assert_eq!(entries.len(), 18);

        assert_eq!(entries[0].spine_index, 1);
        assert_eq!(entries[0].fragment.as_deref(), Some("pgepubid00000"));
        assert_eq!(entries[1].label, "Contents");
        assert_eq!(entries[1].spine_index, 1);

        for entry in &entries[2..=5] {
            assert_eq!(
                entry.spine_index, 2,
                "{:?} is in the same document as its siblings",
                entry.label
            );
        }

        assert!(
            !entries.iter().any(|entry| entry.spine_index == 0),
            "the cover is in the spine but in no toc entry"
        );

        assert_eq!(entries[17].spine_index, 14);
    }

    #[test]
    fn the_current_chapter_is_the_first_entry_naming_its_document() {
        let (epub, docs) =
            epub::open_with_spine(Path::new(crate::TEST_BOOK)).expect("open fixture book");

        let entries = toc_entries(&epub, &docs);
        let label = |spine_index| {
            entry_index_for_spine(&entries, spine_index).map(|index| entries[index].label.as_str())
        };

        assert_eq!(label(1), Some("The Adventures of Sherlock Holmes"));
        assert_eq!(label(2), Some("I. A SCANDAL IN BOHEMIA"));
        assert_eq!(label(14), Some("THE FULL PROJECT GUTENBERG™ LICENSE"));

        assert_eq!(label(0), None);
    }

    #[test]
    fn a_document_the_toc_skips_keeps_the_preceding_entry() {
        let entry = |label: &str, spine_index| TocEntry {
            label: label.to_string(),
            depth: 0,
            spine_index,
            fragment: None,
        };
        let entries = vec![entry("One", 1), entry("Two", 4)];
        let label = |spine_index| {
            entry_index_for_spine(&entries, spine_index).map(|index| entries[index].label.as_str())
        };

        assert_eq!(label(2), Some("One"));
        assert_eq!(label(3), Some("One"));
        assert_eq!(label(4), Some("Two"));
        assert_eq!(label(5), Some("Two"));

        assert_eq!(label(0), None);
    }

    #[test]
    fn an_entry_naming_no_spine_item_is_dropped() {
        let (epub, docs) =
            epub::open_with_spine(Path::new(crate::TEST_BOOK)).expect("open fixture book");

        let entries = toc_entries(&epub, &docs[..3]);

        assert_eq!(entries.len(), 6);
        assert_eq!(entries[5].label, "III.");
    }
}
