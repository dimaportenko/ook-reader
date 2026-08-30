use std::{path::PathBuf, rc::Rc};

use dioxus::prelude::*;
use dioxus_primitives::ContentAlign;
use rbook::Epub;

use crate::{
    clock::now_secs,
    epub,
    library::{self, Book, Library},
    ui::{
        components::{
            icon::{self, Icon},
            popover::{PopoverContent, PopoverRoot},
        },
        OrLog,
    },
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
    let mut import_status = use_signal(|| None::<String>);

    let import = use_callback({
        let library = Rc::clone(&library);

        move |sources: Vec<PathBuf>| {
            if sources.is_empty() {
                return;
            }

            let summary = library.add_all(&sources, now_secs());

            refresh_books(&library, books, status);
            import_status.set(Some(match summary.failed {
                0 => format!("Imported {} books", summary.added),
                failed => format!("Imported {} books, {failed} failed", summary.added),
            }));
        }
    });

    rsx! {
        div {
            div {
                class: "library-books__actions",
                PopoverRoot {
                    is_modal: false,
                    open: import_status().is_some(),
                    on_open_change: move |open: bool| {
                        if !open {
                            import_status.set(None);
                        }
                    },
                    ImportPicker { import }
                    if let Some(message) = import_status() {
                        PopoverContent {
                            align: ContentAlign::End,
                            role: "status",
                            "{message}"
                        }
                    }
                }
                button {
                    class: "icon-button",
                    aria_label: "Edit library",
                    Icon { icon: icon::EDIT }
                }
            }
            ul {
                class: "library-books__list",
                for book in books() {
                    li {
                        class: "library-books__item",
                        key: "{book.id}",

                        button {
                            class: "book-cover",
                            aria_label: "Open {book.title}",
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

#[cfg(target_os = "ios")]
#[component]
fn ImportPicker(import: Callback<Vec<PathBuf>>) -> Element {
    let window = crate::renderer::use_window();

    rsx! {
        button {
            class: "icon-button",
            aria_label: "Add book",
            onclick: move |_| {
                crate::document_picker::pick_epubs(&window.window, move |sources| import.call(sources))
            },
            Icon { icon: icon::ADD }
        }
    }
}

#[cfg(not(target_os = "ios"))]
#[component]
fn ImportPicker(import: Callback<Vec<PathBuf>>) -> Element {
    rsx! {
        label {
            class: "icon-button",
            aria_label: "Add book",

            Icon { icon: icon::ADD }

            input {
                class: "hidden",
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
