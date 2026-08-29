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

const ROOT_THEME_JS: &str = include_str!("web/assets/root-theme.js");

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

    use_effect(move || {
        let push = document::eval(ROOT_THEME_JS);
        _ = push.send(settings().vars());
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
    fn the_theme_paints_the_safe_area_strip() {
        let canvas = MAIN_CSS_SOURCE
            .split('}')
            .find(|rule| rule.contains("var(--USER__backgroundColor)"))
            .expect("nothing outside the reader's own box carries the theme");

        assert!(
            canvas
                .split_once('{')
                .is_some_and(|(selector, _)| selector.trim() == "html"),
            "the theme is painted inside the inset box, so the strip the padding \
             leaves keeps the canvas default",
        );

        assert!(
            settings::Settings::default().vars().starts_with(":root {"),
            "the canvas can only read a variable declared at the document root",
        );

        assert!(
            ROOT_THEME_JS.contains("getElementById"),
            "a push per settings change appends a new <style> every time unless it \
             finds the one it wrote last",
        );
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

    #[test]
    fn every_backdrop_filter_is_paired_with_its_webkit_spelling() {
        let prefixed = MAIN_CSS_SOURCE.matches("-webkit-backdrop-filter:").count();
        let all = MAIN_CSS_SOURCE.matches("backdrop-filter:").count();

        assert!(prefixed > 0, "the glass has no blur to speak of");
        assert_eq!(
            all,
            prefixed * 2,
            "WebKit is the engine that can refuse, and it is the one an unprefixed \
             declaration is invisible on",
        );
    }

    #[test]
    fn the_glass_fill_is_declared_after_the_button_it_overrides() {
        let button = MAIN_CSS_SOURCE
            .find(".icon-button {")
            .expect("the primitive the glass composes onto");
        let glass = MAIN_CSS_SOURCE
            .find(".glass {")
            .expect("the material itself");

        assert!(
            button < glass,
            "one class each is a specificity tie, so only source order decides \
             which background-color the button ends up with",
        );
    }

    #[test]
    fn a_surface_can_set_its_own_blur_radius() {
        let (_, after) = MAIN_CSS_SOURCE
            .split_once(".glass {")
            .expect("the material itself");
        let block = after.split_once('}').expect("an unclosed rule").0;

        assert!(
            !block.contains("--glass-blur:"),
            "a default declared inside .glass beats every consumer that declares \
             the same knob earlier at one class of specificity, so .icon-button's \
             radius never reaches the filter",
        );
    }

    #[test]
    fn transparency_can_be_turned_off_without_turning_the_chrome_off() {
        let (_, reduced) = MAIN_CSS_SOURCE
            .split_once("@media (prefers-reduced-transparency: reduce)")
            .expect("a reading app owes the setting an answer");

        assert!(
            reduced.contains("backdrop-filter: none"),
            "leaving the blur on is the whole thing the setting asks you not to do",
        );
        assert!(
            reduced.contains("--glass-fallback"),
            "each surface names its own opaque colour, or they all collapse to one",
        );
        assert!(
            reduced.contains("background-image: none"),
            "a highlight raking across an opaque panel is the lit-glass cue the \
             setting asks you to drop, and losing the blur does not remove it",
        );
    }

    #[test]
    fn the_specular_angle_is_registered_with_every_descriptor_it_needs() {
        let (_, property) = MAIN_CSS_SOURCE
            .split_once("@property --glass-angle {")
            .expect("an unregistered custom property is a token string, not an angle");
        let block = property.split_once('}').expect("an unclosed rule").0;

        for descriptor in ["syntax:", "inherits:", "initial-value:"] {
            assert!(
                block.contains(descriptor),
                "an @property rule missing {descriptor} is dropped whole, and the \
                 gradient reading it then goes invalid at computed-value time",
            );
        }

        let (_, glass) = MAIN_CSS_SOURCE
            .split_once(".glass {")
            .expect("the material itself");

        assert!(
            glass
                .split_once('}')
                .expect("an unclosed rule")
                .0
                .contains("var(--glass-angle)"),
            "a registered property nothing reads is a no-op",
        );
    }
}
