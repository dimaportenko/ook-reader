use dioxus::prelude::*;

use crate::epub::{self, Locator};

#[derive(Debug, PartialEq)]
enum Seek {
    First,
    Last,
}

#[derive(Debug, PartialEq)]
enum Nav {
    Stay,
    Page(usize),
    Chapter { index: usize, seek: Seek },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum Pending {
    #[default]
    Nothing,
    Fragment(String),
    LastPage,
}

impl Pending {
    pub(crate) fn fragment(self) -> Option<String> {
        match self {
            Pending::Fragment(fragment) => Some(fragment),
            _ => None,
        }
    }

    pub(crate) fn is_settling(&self) -> bool {
        *self != Pending::Nothing
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Phase {
    #[default]
    Loading,
    Ready,
}

#[derive(Store, Default)]
pub(crate) struct ReaderData {
    pub(crate) chapter: usize,
    pub(crate) page: usize,
    pub(crate) page_count: usize,
    pub(crate) pending: Pending,
    pub(crate) phase: Phase,
}

pub(crate) fn chapter_is_hidden(phase: Phase, pending: &Pending) -> bool {
    phase == Phase::Loading || pending.is_settling()
}

#[derive(Clone, Copy)]
pub(crate) struct ReaderState {
    pub(crate) data: Store<ReaderData>,
    pub(crate) chapter_count: usize,
}

pub(crate) const SELECTOR_FRAGMENT_PREFIX: &str = "ook-sel:";

fn restored_data(start: Option<Locator>, chapter_count: usize) -> ReaderData {
    match start {
        Some(locator) if locator.spine_index < chapter_count => ReaderData {
            chapter: locator.spine_index,
            pending: Pending::Fragment(format!("{SELECTOR_FRAGMENT_PREFIX}{}", locator.selector)),
            ..Default::default()
        },
        _ => ReaderData::default(),
    }
}

pub(crate) fn use_reader_state(chapter_count: usize, start: Option<Locator>) -> ReaderState {
    ReaderState {
        data: use_store(move || restored_data(start, chapter_count)),
        chapter_count,
    }
}

impl ReaderState {
    pub(crate) fn page_prev(self) {
        let (page, chapter) = (self.data.page(), self.data.chapter());
        self.apply(on_prev(page(), chapter()));
    }

    pub(crate) fn page_next(self) {
        let (page, page_count, chapter) = (
            self.data.page(),
            self.data.page_count(),
            self.data.chapter(),
        );
        self.apply(on_next(page(), page_count(), chapter(), self.chapter_count));
    }

    fn apply(self, nav: Nav) {
        let (mut page, mut chapter) = (self.data.page(), self.data.chapter());
        match nav {
            Nav::Stay => {}
            Nav::Page(p) => page.set(p),
            Nav::Chapter {
                index,
                seek: Seek::First,
            } => {
                page.set(0);
                chapter.set(index);
                self.data.phase().set(Phase::Loading);
            }
            Nav::Chapter {
                index,
                seek: Seek::Last,
            } => {
                chapter.set(index);
                self.data.pending().set(Pending::LastPage);
                self.data.phase().set(Phase::Loading);
            }
        }
    }

    pub(crate) fn follow_link(self, target: epub::LinkTarget) {
        if *self.data.chapter().peek() != target.spine_index {
            self.data.phase().set(Phase::Loading);
        }
        self.data.chapter().set(target.spine_index);
        self.data.page().set(0);
        self.data.pending().set(match target.fragment {
            Some(fragment) => Pending::Fragment(fragment),
            None => Pending::Nothing,
        });
    }

    pub(crate) fn on_scroll(self, p: usize) {
        let mut pending = self.data.pending();
        self.data.page().set(p);
        if matches!(pending(), Pending::Fragment(_)) {
            pending.set(Pending::Nothing);
        }
    }

    pub(crate) fn on_pages(self, pages: usize) {
        let (mut page, mut pending) = (self.data.page(), self.data.pending());
        self.data.page_count().set(pages);
        if matches!(pending(), Pending::LastPage) {
            page.set(pages.saturating_sub(1));
            pending.set(Pending::Nothing);
        }
    }

    pub(crate) fn on_reflow(self, page: usize) {
        self.data.page().set(page);
    }

    pub(crate) fn on_ready(self) {
        self.data.phase().set(Phase::Ready);
    }
}

fn on_next(page: usize, page_count: usize, chapter: usize, chapter_count: usize) -> Nav {
    if page_count > 0 && page + 1 < page_count {
        Nav::Page(page + 1)
    } else if chapter + 1 < chapter_count {
        Nav::Chapter {
            index: chapter + 1,
            seek: Seek::First,
        }
    } else {
        Nav::Stay
    }
}

fn on_prev(page: usize, chapter: usize) -> Nav {
    if page > 0 {
        Nav::Page(page - 1)
    } else if chapter > 0 {
        Nav::Chapter {
            index: chapter - 1,
            seek: Seek::Last,
        }
    } else {
        Nav::Stay
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn page_nav_rolls_over_chapter_boundaries() {
        assert_eq!(on_next(0, 3, 0, 15), Nav::Page(1));
        assert_eq!(on_prev(2, 3), Nav::Page(1));

        assert_eq!(
            on_next(2, 3, 0, 15),
            Nav::Chapter {
                index: 1,
                seek: Seek::First
            }
        );
        assert_eq!(on_next(2, 3, 14, 15), Nav::Stay);

        assert_eq!(
            on_prev(0, 3),
            Nav::Chapter {
                index: 2,
                seek: Seek::Last
            }
        );
        assert_eq!(on_prev(0, 0), Nav::Stay);

        assert_eq!(
            on_next(0, 0, 0, 15),
            Nav::Chapter {
                index: 1,
                seek: Seek::First
            }
        );
    }

    #[test]
    fn a_stored_position_seeds_the_chapter_and_a_selector_fragment() {
        let locator = Locator {
            spine_index: 8,
            selector: "body > div:nth-child(1) > p:nth-child(215)".to_string(),
        };

        let data = restored_data(Some(locator), 24);
        assert_eq!(data.chapter, 8);
        assert_eq!(
            data.pending,
            Pending::Fragment("ook-sel:body > div:nth-child(1) > p:nth-child(215)".into())
        );
        // The page is deliberately *not* restored. It is derived from the window
        // size, so it is recomputed: `fragment-scroll.js` resolves the selector
        // and reports the page back over `ook-scroll`.
        assert_eq!(data.page, 0);

        // No stored position — start at the top of the book.
        let fresh = restored_data(None, 24);
        assert_eq!(fresh.chapter, 0);
        assert_eq!(fresh.pending, Pending::Nothing);

        // A spine index past the end falls back to the start rather than seeding
        // an index that `docs[chapter()]` would panic on. Re-import keeps the row
        // id and replaces the bytes, so a stored index can outlive the spine it
        // named.
        let stale = Locator {
            spine_index: 24,
            selector: "body > p:nth-child(3)".to_string(),
        };
        let data = restored_data(Some(stale), 24);
        assert_eq!(data.chapter, 0);
        assert_eq!(data.pending, Pending::Nothing);
    }

    #[test]
    fn the_fragment_prefix_matches_the_one_the_asset_looks_for() {
        // Rust builds this prefix, `fragment-scroll.js` tests for it, and no
        // compiler checks a string that crosses a language boundary. Same guard
        // as `the_loader_and_the_cleanup_agree_on_where_the_blob_url_lives`.
        assert!(crate::web::assets::INJECTED_ASSETS.contains(SELECTOR_FRAGMENT_PREFIX));
    }
}
