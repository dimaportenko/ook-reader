#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::prelude::*;

mod clock;
mod config;
mod db;
mod epub;
mod library;
mod nav;
mod settings;
mod ui;
mod web;
mod window;

use library::Library;

use crate::{
    config::Config,
    db::Db,
    ui::{
        library::{ImportControl, LibraryBooks, OpenBook},
        reader::Reader,
        OrLog,
    },
};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const DIOXUS_PRIMITIVES_CSS: Asset = asset!("/assets/dx-components-theme.css");

#[cfg(test)]
pub(crate) const TEST_BOOK: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/book/pg1661-adventures-of-sherlock-holmes.epub"
);

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // TODO: refactor expect to show some user error before app close (low priority)
    let config = use_hook(|| Rc::new(Config::new().expect("a home directory should exist")));
    let db = use_hook(|| Rc::new(Db::open(&config.app_dir).expect("Open database file")));
    let library = use_hook(|| Rc::new(Library::new(db.clone(), &config.books_dir)));
    let settings = use_hook(|| {
        Signal::new(
            db.settings()
                .or_log("read your settings")
                .flatten()
                .unwrap_or_default(),
        )
    });
    let (books, status) = use_hook(|| match library.list() {
        Ok(books) => (Signal::new(books), Signal::new(None)),
        Err(error) => (
            Signal::new(Vec::new()),
            Signal::new(Some(format!("Could not load your library: {error}"))),
        ),
    });
    let open_book = use_signal(|| None::<OpenBook>);

    let desktop = dioxus::desktop::use_window();
    use_hook(move || window::remember_frame(&desktop.window));

    use_context_provider(|| library.clone());
    use_context_provider(|| books);
    use_context_provider(|| status);
    use_context_provider(|| open_book);
    use_context_provider(|| settings);

    use_effect({
        let db = db.clone();
        move || {
            _ = db.save_settings(&settings()).or_log("save your settings");
        }
    });

    epub::use_register_covers_handler(config.books_dir.clone());

    rsx! {
        document::Link {
            rel: "icon",
            href: FAVICON,
        }
        document::Link {
            rel: "stylesheet",
            href: MAIN_CSS,
        }
        document::Link {
            rel: "stylesheet",
            href: DIOXUS_PRIMITIVES_CSS,
        }

        if let Some(book) = open_book() {
            Reader {
                key: "{book.id}",
                book,
            }
        } else {
            LibraryBooks {}
            ImportControl {}
        }
    }
}
