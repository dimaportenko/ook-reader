use std::rc::Rc;

use dioxus::prelude::*;
use rbook::Epub;

use crate::{
    epub::{self, SpineDoc},
    library::{self, Book, Library},
};

static PLACEHOLDER_2: Asset = asset!("/assets/books/placeholder-2.jpg");

#[derive(Clone)]
pub(crate) struct OpenBook {
    pub(crate) id: i64,
    pub(crate) title: String,
    pub(crate) epub: Rc<Epub>,
    pub(crate) docs: Rc<Vec<epub::SpineDoc>>,
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
    let mut open_status = use_signal(|| None::<String>);

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
                                let id = book.id;
                                let title = book.title;
                                let path = book.path;

                                move |_| {
                                    let result = open_epub(std::path::Path::new(&path));
                                    match result
                                    {
                                        Ok((epub, docs)) => {
                                            open_status.set(None);
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
                                        Err(error) => open_status.set(Some(format!("Open failed: {error}"))),
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

                                move |_| {
                                    if library.remove(id).is_ok() {
                                        refresh_books(&library, books);
                                    }
                                }
                            },
                            "Remove"
                        }

                    }
                }

            }
        }
        if let Some(status) = open_status() {
            p {
                "{status}"
            }
        }
    }
}

#[component]
fn BookCover(book: Book) -> Element {
    rsx! {
        if let Some(name) = book.get_book_cover_name() {
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
    let mut status = use_signal(|| None::<String>);

    rsx! {
        div {
            style: "padding: 8px; display: flex; gap: 8px; align-items: center;",

            label {
                "Import EPUB "

                input {
                    r#type: "file",
                    accept: ".epub",
                    onchange: move |event| {
                        let Some(file) = event.files().into_iter().next() else {
                            return;
                        };
                        match library.add_from_path(&file.path()) {
                            Ok(book) => {
                                status.set(Some(format!("Imported: {}", book.title)));
                                refresh_books(&library, books);
                            }
                            Err(error) => status.set(Some(format!("Import failed: {error}"))),
                        }
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

fn refresh_books(library: &Library, mut books: Signal<Vec<Book>>) {
    if let Ok(list) = library.list() {
        books.set(list);
    }
}

fn open_epub(path: &std::path::Path) -> Result<(Epub, Vec<SpineDoc>), Box<dyn std::error::Error>> {
    let epub = Epub::open(path)?;
    let docs = epub::load_spine(&epub)?;
    Ok((epub, docs))
}
