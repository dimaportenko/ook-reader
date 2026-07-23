#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::prelude::*;

mod epub;
mod library;
mod nav;
mod ui;

use library::Library;

use crate::ui::library::{ImportControl, LibraryBooks, OpenBook};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

#[cfg(test)]
pub(crate) const BOOK: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/book/pg1661-adventures-of-sherlock-holmes.epub"
);

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let library = use_hook(|| Rc::new(Library::open_default()));
    let books = use_signal(|| library.list().unwrap_or(vec![]));
    let open_book = use_signal(|| None::<OpenBook>);

    use_context_provider(|| library.clone());
    use_context_provider(|| books);
    use_context_provider(|| open_book);

    epub::use_register_covers_handler(library.books_dir().to_path_buf());

    rsx! {
        document::Link {
            rel: "icon",
            href: FAVICON,
        }
        document::Link {
            rel: "stylesheet",
            href: MAIN_CSS,
        }

        if let Some(book) = open_book() {
            ui::reader::Reader {
                key: "{book.id}",
                book,
            }
        } else {
            LibraryBooks {}
            ImportControl {}
        }
    }
}
