#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::prelude::*;

mod clock;
mod config;
mod db;
#[cfg(target_os = "ios")]
mod document_picker;
mod epub;
mod library;
mod nav;
mod settings;
mod toc;
mod ui;
mod web;
mod window;

#[cfg(feature = "desktop")]
pub(crate) use dioxus::desktop as renderer;

#[cfg(all(feature = "mobile", not(feature = "desktop")))]
pub(crate) use dioxus::mobile as renderer;

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

const VIEWPORT: &str =
    "width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover";

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

    let desktop = crate::renderer::use_window();
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
        document::Meta {
            name: "viewport",
            content: VIEWPORT,
        }
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

#[cfg(test)]
mod test {
    use super::*;

    const MAIN_CSS_SOURCE: &str = include_str!("../assets/main.css");

    #[test]
    fn the_safe_area_is_only_paid_out_to_a_viewport_that_covers_it() {
        assert!(
            VIEWPORT.contains("viewport-fit=cover"),
            "without it every env(safe-area-inset-*) below resolves to 0",
        );
        assert_eq!(
            MAIN_CSS_SOURCE.matches("env(safe-area-inset-").count(),
            4,
            "the app's box is inset on all four edges or on none",
        );
    }

    #[test]
    fn the_replacement_viewport_restates_what_it_overrides() {
        assert!(VIEWPORT.contains("width=device-width"));
        assert!(VIEWPORT.contains("initial-scale=1.0"));
        assert!(VIEWPORT.contains("user-scalable=no"));
    }

    #[test]
    fn a_full_height_screen_is_measured_inside_the_inset_box() {
        assert!(
            !MAIN_CSS_SOURCE.contains("100vh"),
            "vh is the whole display, padding and all, so a 100vh screen hangs the \
             insets off the bottom edge",
        );
        assert!(
            MAIN_CSS_SOURCE.contains("#main"),
            "the Dioxus mount point has to carry the height too, or the percentage \
             resolves against an auto-height ancestor and the screen collapses",
        );
    }
}
