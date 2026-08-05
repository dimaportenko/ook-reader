use std::rc::Rc;

use dioxus::prelude::*;
use rbook::Epub;

use crate::{
    epub,
    library::{self, Book, Library},
    ui::OrLog,
};

static PLACEHOLDER_2: Asset = asset!("/assets/books/placeholder-2.jpg");

#[derive(Clone)]
pub(crate) struct OpenBook {
    pub(crate) id: i64,
    pub(crate) title: String,
    pub(crate) epub: Rc<Epub>,
    pub(crate) docs: Rc<Vec<String>>,
}

impl PartialEq for OpenBook {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[component]
pub(crate) fn LibraryBooks() -> Element {
    let library = use_context::<Rc<Library>>();
    let books = use_context::<Signal<Vec<library::Book>>>();
    let mut open_book = use_context::<Signal<Option<OpenBook>>>();
    let mut status = use_context::<Signal<Option<String>>>();

    rsx! {
        div {
            ul {
                class: "library-books__list",
                for book in books() {
                    li {
                        class: "library-books__item",
                        key: "{book.id}",

                        button {
                            class: "book-cover",
                            onclick: {
                                let library = Rc::clone(&library);
                                let id = book.id;
                                let title = book.title;
                                let path = book.path;

                                move |_| {
                                    match open_epub(std::path::Path::new(&path))
                                    {
                                        Ok((epub, docs)) => {
                                            status.set(None);
                                            library
                                                .touch_opened(id, library::now_secs())
                                                .or_log("update the last-opened time");
                                            refresh_books(&library, books, status);
                                            open_book
                                                .set(
                                                    Some(OpenBook {
                                                        id,
                                                        title: title.clone(),
                                                        epub: Rc::new(epub),
                                                        docs: Rc::new(docs),
                                                    }),
                                                );
                                        }
                                        Err(error) => status.set(Some(format!("Open failed: {error}"))),
                                    }
                                }
                            },

                            BookCover {
                                book: book.clone(),
                            }
                        }

                        button {
                            onclick: {
                                let library = Rc::clone(&library);
                                let id = book.id;

                                move |_| match library.remove(id) {
                                    Ok(_) => refresh_books(&library, books, status),
                                    Err(error) => {
                                        status.set(Some(format!("Remove failed: {error}")))
                                    }
                                }
                            },
                            "Remove"
                        }

                    }
                }

            }
        }
        if let Some(message) = status() {
            p {
                "{message}"
            }
        }
    }
}

#[component]
fn BookCover(book: Book) -> Element {
    rsx! {
        if let Some(name) = book.cover_name() {
            div {
                class: "book-cover__container",
                img {
                    class: "book-cover__img",
                    src: "/covers/{name}",
                }
            }
        } else {
            div {
                class: "book-cover__container",
                img {
                    class: "book-cover__img",
                    src: PLACEHOLDER_2,
                }
                div {
                    class: "book-cover__placeholder",
                    span {
                        class: "book-cover__placeholder-title",
                        "{book.title}"
                    }
                    if let Some(author) = book.author.as_deref() {
                        span {
                            class: "book-cover__placeholder-author",
                            "{author}"
                        }
                    }
                }
            }
        }

    }
}

#[component]
pub(crate) fn ImportControl() -> Element {
    let library = use_context::<Rc<Library>>();
    let books = use_context::<Signal<Vec<library::Book>>>();
    let library_status = use_context::<Signal<Option<String>>>();
    let mut status = use_signal(|| None::<String>);

    rsx! {
        div {
            style: "padding: 8px; display: flex; gap: 8px; align-items: center;",

            label {
                "Import EPUB "

                input {
                    r#type: "file",
                    accept: ".epub",
                    multiple: true,
                    onchange: move |event| {
                        let files = event.files();
                        if files.is_empty() {
                            return;
                        }

                        let mut imported = 0usize;
                        let mut failed = 0usize;
                        let added_at = library::now_secs();

                        for file in files {
                            match library.add_from_path(&file.path(), added_at) {
                                Ok(_) => imported += 1,
                                Err(_) => failed += 1,
                            }
                        }

                        status
                            .set(
                                match failed {
                                    0 => format!("Imported {imported} books").into(),
                                    _ => format!("Imported {imported} books, {failed} failed").into(),
                                },
                            );

                        refresh_books(&library, books, library_status);
                    },
                }
            }

            if let Some(message) = status() {
                span {
                    "{message}"
                }
            }
        }
    }
}

fn refresh_books(
    library: &Library,
    mut books: Signal<Vec<Book>>,
    mut status: Signal<Option<String>>,
) {
    match library.list() {
        Ok(list) => {
            books.set(list);
            status.set(None);
        }
        Err(error) => status.set(Some(format!("Could not refresh your library: {error}"))),
    }
}

fn open_epub(path: &std::path::Path) -> Result<(Epub, Vec<String>), library::Error> {
    let epub = Epub::open(path)?;
    let docs = epub::spine_hrefs(&epub)?;
    Ok((epub, docs))
}
