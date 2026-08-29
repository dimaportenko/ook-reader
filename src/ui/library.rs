use std::{path::PathBuf, rc::Rc};

use dioxus::prelude::*;
use rbook::Epub;

use crate::{
    clock::now_secs,
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
                                let file_name = book.file_name;

                                move |_| {
                                    match epub::open_with_spine(&library.book_path(&file_name))
                                    {
                                        Ok((epub, docs)) => {
                                            status.set(None);
                                            library
                                                .touch_opened(id, now_secs())
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
        if let Some(name) = &book.cover_name {
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

    let import = use_callback(move |sources: Vec<PathBuf>| {
        if sources.is_empty() {
            return;
        }

        let summary = library.add_all(&sources, now_secs());

        status.set(Some(match summary.failed {
            0 => format!("Imported {} books", summary.added),
            failed => format!("Imported {} books, {failed} failed", summary.added),
        }));

        refresh_books(&library, books, library_status);
    });

    rsx! {
        div {
            style: "padding: 8px; display: flex; gap: 8px; align-items: center;",

            ImportPicker { import }

            if let Some(message) = status() {
                span {
                    "{message}"
                }
            }
        }
    }
}

#[cfg(target_os = "ios")]
#[component]
fn ImportPicker(import: Callback<Vec<PathBuf>>) -> Element {
    let window = crate::renderer::use_window();

    rsx! {
        button {
            onclick: move |_| {
                crate::document_picker::pick_epubs(&window.window, move |sources| import.call(sources))
            },
            "Import EPUB"
        }
    }
}

#[cfg(not(target_os = "ios"))]
#[component]
fn ImportPicker(import: Callback<Vec<PathBuf>>) -> Element {
    rsx! {
        label {
            "Import EPUB "

            input {
                r#type: "file",
                accept: ".epub",
                multiple: true,
                onchange: move |event| {
                    import.call(event.files().iter().map(|file| file.path()).collect());
                },
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
