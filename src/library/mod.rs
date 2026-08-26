use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use rbook::Epub;

use crate::{
    db::{Db, NewBook},
    epub::{self, Locator},
    library::files::BookFiles,
};

mod files;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("file error: {0}")]
    Io(#[from] std::io::Error),

    #[error("could not read the EPUB: {0}")]
    Ebook(#[from] rbook::ebook::errors::EbookError),
}

pub(crate) use crate::db::Book;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ImportSummary {
    pub(crate) added: usize,
    pub(crate) failed: usize,
}

pub(crate) struct Library {
    db: Rc<Db>,
    files: BookFiles,
}

impl Library {
    pub(crate) fn new(db: Rc<Db>, books_dir: &Path) -> Self {
        Self {
            db,
            files: BookFiles::new(books_dir.to_path_buf()),
        }
    }

    pub(crate) fn add_all(&self, sources: &[PathBuf], now: i64) -> ImportSummary {
        let mut summary = ImportSummary::default();

        for source in sources {
            match self.add_from_path(source, now) {
                Ok(_) => summary.added += 1,
                Err(_) => summary.failed += 1,
            }
        }

        summary
    }

    pub(crate) fn add_from_path(&self, source_path: &Path, now: i64) -> Result<Book, Error> {
        let source_path = source_path.canonicalize()?;
        let source_path_text = source_path.to_string_lossy().into_owned();

        let previous = self.db.managed_paths_for_source(&source_path_text)?;

        let managed_path = self.files.import(&source_path)?;

        let mut cover_path: Option<String> = None;

        let result = (|| -> Result<Book, Error> {
            let epub = Epub::open(&managed_path)?;
            let meta = epub::read_metadata(&epub);
            let managed_path_text = managed_path.to_string_lossy().into_owned();

            cover_path = meta.cover.as_ref().and_then(|cover| {
                let ext = epub::extension_for(&cover.media_type)?;
                self.files.write_cover(&managed_path, ext, &cover.bytes)
            });

            let book = self.db.upsert_book(NewBook {
                path: &managed_path_text,
                source_path: &source_path_text,
                title: &meta.title,
                author: meta.author.as_deref(),
                cover_path: cover_path.as_deref(),
                added_at: now,
            })?;

            Ok(book)
        })();

        match &result {
            Err(_) => {
                self.files.remove(&managed_path);
                if let Some(path) = &cover_path {
                    self.files.remove(Path::new(path));
                }
            }
            Ok(_) => {
                if let Some((previous_path, previous_cover)) = previous {
                    self.files.remove(Path::new(&previous_path));
                    if let Some(cover) = previous_cover {
                        self.files.remove(Path::new(&cover));
                    }
                }
            }
        }

        result
    }

    pub(crate) fn remove(&self, id: i64) -> Result<bool, Error> {
        let removed = self.db.delete_book(id)?;

        if let Some((removed_path, removed_cover)) = removed {
            self.files.remove(Path::new(&removed_path));
            if let Some(cover) = removed_cover {
                self.files.remove(Path::new(&cover));
            }
            return Ok(true);
        };

        Ok(false)
    }

    pub(crate) fn list(&self) -> Result<Vec<Book>, Error> {
        Ok(self.db.list_books()?)
    }

    pub(crate) fn touch_opened(&self, id: i64, now: i64) -> Result<bool, Error> {
        Ok(self.db.touch_opened(id, now)?)
    }

    pub(crate) fn save_position(
        &self,
        book_id: i64,
        locator: &Locator,
        now: i64,
    ) -> Result<(), Error> {
        Ok(self.db.save_position(book_id, locator, now)?)
    }

    pub(crate) fn position(&self, book_id: i64) -> Result<Option<Locator>, Error> {
        Ok(self.db.position(book_id)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn add_then_list_round_trips_books() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, first_source, books_dir) = library_with_source(&dir);
        let second_source = dir.path().join("holmes-second-source.epub");
        std::fs::copy(crate::TEST_BOOK, &second_source).expect("second fixture source");

        let first = library
            .add_from_path(&first_source, 1_000)
            .expect("first import");
        let second = library
            .add_from_path(&second_source, 2_000)
            .expect("second import");

        // Distinct source paths are distinct books, with metadata read from the file.
        assert_ne!(first.id, second.id);
        assert!(Path::new(&first.path).starts_with(&books_dir));
        assert!(first.title.contains("Sherlock Holmes"));
        assert!(first.author.as_deref().unwrap_or("").contains("Doyle"));

        // Both sources are the same fixture, so titles tie and ORDER BY title
        // leaves their relative order unspecified — assert contents, not order.
        let books = library.list().expect("list succeeds");
        assert_eq!(books.len(), 2);
        assert!(books.contains(&first));
        assert!(books.contains(&second));
    }

    #[test]
    fn file_backed_library_survives_reopen_and_reimport_is_idempotent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, books_dir) = library_with_source(&dir);
        let first = library.add_from_path(&source, 1_000).expect("first import");
        drop(library);

        let db = Db::open(&dir).expect("open db");
        let library = Library::new(Rc::new(db), &books_dir);
        let second = library
            .add_from_path(&source, 2_000)
            .expect("second import");
        let books = library.list().expect("list succeeds");

        assert_eq!(second.id, first.id);
        assert_eq!(books, vec![second]);
    }

    #[test]
    fn remove_drops_the_row_and_is_a_noop_for_unknown_ids() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, first_source, _) = library_with_source(&dir);
        let second_source = dir.path().join("holmes-second-source.epub");
        std::fs::copy(crate::TEST_BOOK, &second_source).expect("second fixture source");
        let first = library
            .add_from_path(&first_source, 1_000)
            .expect("first import");
        let second = library
            .add_from_path(&second_source, 2_000)
            .expect("second import");

        // Remove by the DB-assigned id, not by path.
        let removed = library.remove(first.id).expect("remove succeeds");
        assert!(removed, "expected an existing row to report true");

        let books = library.list().expect("list succeeds");
        assert_eq!(books, vec![second]);

        // Unknown id: no error, no change, reports false.
        let removed_again = library.remove(first.id).expect("missing id is Ok(false)");
        assert!(!removed_again);
        assert_eq!(library.list().expect("list still one").len(), 1);
    }

    #[test]
    fn import_opens_from_managed_copy_after_source_is_deleted() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, books_dir) = library_with_source(&dir);
        let added = library
            .add_from_path(&source, 1_000)
            .expect("import succeeds");

        assert!(std::path::Path::new(&added.path).starts_with(&books_dir));
        assert_ne!(std::path::Path::new(&added.path), source.as_path());

        std::fs::remove_file(&source).expect("delete source");
        let epub = rbook::Epub::open(&added.path).expect("managed copy opens");
        let meta = crate::epub::read_metadata(&epub);

        assert!(meta.title.contains("Sherlock Holmes"));
        assert_eq!(added.title, meta.title);
    }

    #[test]
    fn reimport_replaces_the_managed_copy_without_leaking_the_old_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, books_dir) = library_with_source(&dir);
        let first = library.add_from_path(&source, 1_000).expect("first import");
        let second = library.add_from_path(&source, 2_000).expect("reimport");

        // Same logical book, fresh bytes: id stable, managed path replaced.
        assert_eq!(second.id, first.id);
        assert_ne!(second.path, first.path);
        assert!(!Path::new(&first.path).exists());
        assert!(Path::new(&second.path).exists());

        // 2 managed file (epub and cover image) and one row — nothing leaked, nothing duplicated.
        let files = std::fs::read_dir(&books_dir)
            .expect("read books dir")
            .count();
        assert_eq!(files, 2);
        assert_eq!(library.list().expect("list succeeds"), vec![second]);
    }

    #[test]
    fn reimport_repairs_a_missing_managed_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);
        let first = library.add_from_path(&source, 1_000).expect("first import");

        // Simulate a hand-deleted managed file: the row now points at nothing.
        std::fs::remove_file(&first.path).expect("delete managed copy");

        let repaired = library
            .add_from_path(&source, 1_000)
            .expect("reimport repairs");

        assert_eq!(repaired.id, first.id);
        rbook::Epub::open(&repaired.path).expect("repaired copy opens");
    }

    #[test]
    fn remove_deletes_the_row_and_managed_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);

        let added = library
            .add_from_path(&source, 1_000)
            .expect("import succeeds");

        let removed = library.remove(added.id).expect("remove succeeds");

        assert!(removed, "expected an existing row to report true");
        assert!(library.list().expect("list succeeds").is_empty());
        assert!(!Path::new(&added.path).exists(), "managed copy is deleted");
        assert!(source.exists(), "the user's original source is untouched");
    }

    #[test]
    fn remove_succeeds_when_the_managed_copy_is_already_missing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);
        let added = library
            .add_from_path(&source, 1_000)
            .expect("import succeeds");
        // Simulate a hand-deleted managed file: the row now points at nothing.
        std::fs::remove_file(&added.path).expect("delete managed copy");

        let removed = library
            .remove(added.id)
            .expect("missing file is not an error");

        assert!(removed, "a stale row is still removable");
        assert!(library.list().expect("list succeeds").is_empty());
    }

    #[test]
    fn add_all_counts_every_source_and_keeps_going_past_a_bad_one() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);
        let missing = dir.path().join("not-a-book.epub");

        let summary = library.add_all(&[missing, source], 1_000);

        assert_eq!(
            summary,
            ImportSummary {
                added: 1,
                failed: 1
            }
        );
        assert_eq!(library.list().expect("list succeeds").len(), 1);
    }

    fn library_with_source(dir: &tempfile::TempDir) -> (Library, PathBuf, PathBuf) {
        let books_dir = dir.path().join("books");
        std::fs::create_dir_all(&books_dir).expect("books dir");
        let db = Db::open(dir).expect("open db");
        let library = Library::new(Rc::new(db), &books_dir);
        let source = dir.path().join("holmes-source.epub");
        std::fs::copy(crate::TEST_BOOK, &source).expect("fixture source");
        (library, source, books_dir)
    }

    #[test]
    fn import_writes_a_cover_file_next_to_the_managed_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);
        let added = library
            .add_from_path(&source, 1_000)
            .expect("import succeeds");

        let cover_path = added.cover_path.expect("bundled book has a cover");
        assert!(Path::new(&cover_path).starts_with(dir.path().join("books")));
        assert!(Path::new(&cover_path).exists());
        // The stored extension round-trips through the serve-time content-type lookup.
        assert!(crate::epub::content_type_for(&cover_path).starts_with("image/"));
    }

    #[test]
    fn reimport_replaces_the_cover_without_leaking_files() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);

        let first = library.add_from_path(&source, 1_000).expect("first import");
        let second = library.add_from_path(&source, 2_000).expect("reimport");

        let first_cover = first.cover_path.expect("first import has a cover");
        let second_cover = second.cover_path.expect("reimport has a cover");

        // Same logical book, fresh files: the old cover is gone, the new one exists.
        assert_ne!(second_cover, first_cover);
        assert!(!Path::new(&first_cover).exists());
        assert!(Path::new(&second_cover).exists());

        // Exactly one .epub + one cover — nothing leaked, nothing duplicated.
        // (This is the assertion that goes red in the *old* reimport test: its
        // `files == 1` becomes `files == 2` once covers land next to the copies.)
        let files = std::fs::read_dir(dir.path().join("books"))
            .expect("read books dir")
            .count();
        assert_eq!(files, 2);
    }

    #[test]
    fn remove_deletes_the_cover_file_too() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);

        let added = library
            .add_from_path(&source, 1_000)
            .expect("import succeeds");
        let cover_path = added.cover_path.clone().expect("import has a cover");

        let removed = library.remove(added.id).expect("remove succeeds");

        assert!(removed, "expected an existing row to report true");
        assert!(library.list().expect("list succeeds").is_empty());
        assert!(!Path::new(&added.path).exists(), "managed copy is deleted");
        assert!(!Path::new(&cover_path).exists(), "cover file is deleted");
        assert!(source.exists(), "the user's original source is untouched");
    }

    #[test]
    fn import_stamps_added_at_and_reimport_preserves_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);

        let first = library.add_from_path(&source, 1_000).expect("first import");

        assert_eq!(first.added_at, 1_000);
        assert_eq!(
            first.last_opened_at, None,
            "a freshly imported book has never been opened",
        );

        // Same source path → same row, fresh bytes and metadata …
        let second = library.add_from_path(&source, 2_000).expect("reimport");

        assert_eq!(second.id, first.id);
        assert_ne!(second.path, first.path);
        // … but the day it joined the library is not "fresh metadata".
        assert_eq!(second.added_at, 1_000);
    }

    #[test]
    fn opening_a_book_floats_it_to_the_top_of_the_list() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, first_source, _) = library_with_source(&dir);
        let second_source = dir.path().join("holmes-second-source.epub");
        std::fs::copy(crate::TEST_BOOK, &second_source).expect("second fixture source");

        let older = library
            .add_from_path(&first_source, 1_000)
            .expect("first import");
        let newer = library
            .add_from_path(&second_source, 2_000)
            .expect("second import");

        // Nothing opened yet: `added_at` stands in, so the newest import leads.
        let ids: Vec<i64> = library.list().expect("list").iter().map(|b| b.id).collect();
        assert_eq!(ids, vec![newer.id, older.id]);

        // Open the *older* import, later than either import moment.
        let touched = library
            .touch_opened(older.id, 3_000)
            .expect("touch succeeds");
        assert!(touched, "an existing row reports true");

        let books = library.list().expect("list");
        assert_eq!(books[0].id, older.id, "the book you just read leads");
        assert_eq!(books[0].last_opened_at, Some(3_000));
        assert_eq!(
            books[1].last_opened_at, None,
            "the book you didn't open is untouched",
        );

        // Unknown id: no error, no change — same contract as `remove`.
        let missing = library
            .touch_opened(-1, 4_000)
            .expect("missing id is Ok(false)");
        assert!(!missing);
    }

    #[test]
    fn position_round_trips_and_latest_save_wins() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);
        let book = library.add_from_path(&source, 1_000).expect("import");

        assert_eq!(library.position(book.id).expect("empty position"), None);

        let first = Locator {
            spine_index: 2,
            selector: "body > p:nth-child(3)".to_string(),
        };
        library
            .save_position(book.id, &first, 2_000)
            .expect("first save");
        assert_eq!(library.position(book.id).expect("first read"), Some(first));

        let latest = Locator {
            spine_index: 4,
            selector: "body > div:nth-child(2) > p:nth-child(7)".to_string(),
        };
        library
            .save_position(book.id, &latest, 3_000)
            .expect("latest save");
        assert_eq!(
            library.position(book.id).expect("latest read"),
            Some(latest),
        );
    }

    #[test]
    fn removing_a_book_cascades_to_its_position() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);
        let book = library.add_from_path(&source, 1_000).expect("import");
        let locator = Locator {
            spine_index: 2,
            selector: "body > p:nth-child(3)".to_string(),
        };

        library
            .save_position(book.id, &locator, 2_000)
            .expect("save position");
        assert!(library.remove(book.id).expect("remove book"));
        assert_eq!(library.position(book.id).expect("position lookup"), None);
    }

    #[test]
    fn importing_a_file_that_is_not_an_epub_reports_a_matchable_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let books_dir = dir.path().join("books");
        std::fs::create_dir_all(&books_dir).expect("books dir");
        let db = Db::open(&dir).expect("open db");
        let library = Library::new(Rc::new(db), &books_dir);

        let source = dir.path().join("not-a-book.epub");
        std::fs::write(&source, b"definitely not a zip archive").expect("write fixture");

        let error = library.add_from_path(&source, 1_000).unwrap_err();

        assert!(matches!(error, Error::Ebook(_)), "got {error:?}");
        assert!(
            !error.to_string().is_empty(),
            "Display must carry a cause for logs"
        );

        let leftovers = std::fs::read_dir(&books_dir)
            .expect("books dir readable")
            .count();
        assert_eq!(
            leftovers, 0,
            "a failed import must not leak its managed copy"
        );
    }
}
