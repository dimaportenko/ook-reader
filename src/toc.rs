use rbook::Epub;

use crate::epub::{self, LinkTarget};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TocEntry {
    pub(crate) label: String,
    pub(crate) depth: usize,
    pub(crate) target: LinkTarget,
}

pub(crate) fn toc_entries(epub: &Epub, docs: &[String]) -> Vec<TocEntry> {
    let Some(root) = epub.toc().contents() else {
        return Vec::new();
    };

    root.flatten()
        .filter_map(|entry| {
            let href = entry.href()?;

            Some(TocEntry {
                label: entry.label().to_string(),
                depth: entry.depth().saturating_sub(1),
                target: epub::link_target(docs, href.path().as_str(), href.fragment())?,
            })
        })
        .collect()
}

pub(crate) fn entry_index_for_spine(entries: &[TocEntry], spine_index: usize) -> Option<usize> {
    entries
        .iter()
        .position(|entry| entry.target.spine_index == spine_index)
        .or_else(|| {
            entries
                .iter()
                .rposition(|entry| entry.target.spine_index < spine_index)
        })
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use super::*;

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

        assert_eq!(entries[0].target.spine_index, 1);
        assert_eq!(entries[0].target.fragment.as_deref(), Some("pgepubid00000"));
        assert_eq!(entries[1].label, "Contents");
        assert_eq!(entries[1].target.spine_index, 1);

        for entry in &entries[2..=5] {
            assert_eq!(
                entry.target.spine_index, 2,
                "{:?} is in the same document as its siblings",
                entry.label
            );
        }

        assert!(
            !entries.iter().any(|entry| entry.target.spine_index == 0),
            "the cover is in the spine but in no toc entry"
        );

        assert_eq!(entries[17].target.spine_index, 14);
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
            target: LinkTarget {
                spine_index,
                fragment: None,
            },
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
    fn an_entry_carries_the_link_target_its_href_would_have() {
        let (epub, docs) =
            epub::open_with_spine(Path::new(crate::TEST_BOOK)).expect("open fixture book");

        let entries = toc_entries(&epub, &docs);
        let target = &entries[2].target;

        assert_eq!(target.spine_index, 2);
        assert_eq!(target.fragment.as_deref(), Some("pgepubid00002"));

        let href = format!("{}#pgepubid00002", epub::chapter_url(&docs[2]));
        assert_eq!(
            epub::resolve_internal_link(&docs, 0, &href).as_ref(),
            Some(target),
            "a picked entry and a followed link reach `follow_link` the same way"
        );
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
