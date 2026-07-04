use dioxus::prelude::*;

use crate::epub;

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

#[derive(Store, Default)]
pub(crate) struct ReaderData {
    pub(crate) chapter: usize,
    pub(crate) page: usize,
    pub(crate) page_count: usize,
    pub(crate) pending_fragment: Option<String>,
    pub(crate) pending_last: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct ReaderState {
    pub(crate) data: Store<ReaderData>,
    pub(crate) chapter_count: usize,
}

pub(crate) fn use_reader_state(chapter_count: usize) -> ReaderState {
    ReaderState {
        data: use_store(ReaderData::default),
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
            }
            Nav::Chapter {
                index,
                seek: Seek::Last,
            } => {
                chapter.set(index);
                self.data.pending_last().set(true);
            }
        }
    }

    pub(crate) fn chapter_prev(self) {
        let mut chapter = self.data.chapter();
        self.data.page().set(0);
        chapter.set(prev_index(chapter()));
    }

    pub(crate) fn chapter_next(self) {
        let mut chapter = self.data.chapter();
        self.data.page().set(0);
        chapter.set(next_index(chapter(), self.chapter_count));
    }

    pub(crate) fn follow_link(self, target: epub::LinkTarget) {
        self.data.chapter().set(target.spine_index);
        self.data.page().set(0);
        self.data.pending_fragment().set(target.fragment);
    }

    pub(crate) fn on_scroll(self, p: usize) {
        self.data.page().set(p);
        self.data.pending_fragment().set(None);
    }

    pub(crate) fn on_pages(self, pages: usize) {
        let (mut page, mut pending_last) = (self.data.page(), self.data.pending_last());
        self.data.page_count().set(pages);
        if pending_last() {
            page.set(pages.saturating_sub(1));
            pending_last.set(false);
        }
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

fn prev_index(chapter: usize) -> usize {
    chapter.saturating_sub(1)
}

fn next_index(chapter: usize, len: usize) -> usize {
    (chapter + 1).min(len.saturating_sub(1))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn paging_clamps_at_both_ends() {
        let len = 15;

        assert_eq!(next_index(0, len), 1);

        assert_eq!(next_index(len - 1, len), len - 1);

        assert_eq!(prev_index(5), 4);

        assert_eq!(prev_index(0), 0);
    }

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
}
