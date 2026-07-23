#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::prelude::*;
use directories::ProjectDirs;
use rbook::Epub;

mod epub;
mod library;
mod nav;

use library::Library;
use nav::*;

use crate::epub::SpineDoc;

static PLACEHOLDER_2: Asset = asset!("/assets/books/placeholder-2.jpg");
const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const BRIDGE_JS: &str = r#"
            window.addEventListener('message', (e) => {
                if (!e.data) return;
                if (e.data.kind === 'ook-link') {
                    dioxus.send("link:" + e.data.raw);
                }
                if (e.data.kind === 'ook-scroll') {
                    dioxus.send("scroll:" + e.data.page);
                }
                if (e.data.kind === 'ook-pages') {
                    dioxus.send("pages:" + e.data.count);
                }
            });
            "#;

#[cfg(test)]
pub(crate) const BOOK: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/book/pg1661-adventures-of-sherlock-holmes.epub"
);

fn open_library() -> Library {
    let dirs = ProjectDirs::from("com", "dimaportenko", "ook-reader")
        .expect("a home directory should exist");
    let data_dir = dirs.data_dir();
    let books_dir = data_dir.join("books");

    std::fs::create_dir_all(&books_dir).expect("app data dir should be creatable");
    Library::open(data_dir.join("library.sqlite3"), books_dir).expect("library db should open")
}

#[derive(Clone)]
struct OpenBook {
    id: i64,
    title: String,
    epub: Rc<Epub>,
    docs: Rc<Vec<epub::SpineDoc>>,
}

impl PartialEq for OpenBook {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, PartialEq)]
enum BridgeMsg {
    Link(String),
    Scroll(usize),
    Pages(usize),
}

impl BridgeMsg {
    fn parse(msg: &str) -> Option<BridgeMsg> {
        if let Some(href) = msg.strip_prefix("link:") {
            Some(BridgeMsg::Link(href.to_string()))
        } else if let Some(page) = msg.strip_prefix("scroll:") {
            page.parse().ok().map(BridgeMsg::Scroll)
        } else if let Some(page_count) = msg.strip_prefix("pages:") {
            page_count.parse().ok().map(BridgeMsg::Pages)
        } else {
            None
        }
    }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let library = use_hook(|| Rc::new(open_library()));
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

#[component]
fn NavRow(
    label: String,
    on_next: EventHandler<MouseEvent>,
    on_prev: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div {
            style: "display: flex; gap: 8px; padding: 8px; justify-content: center;",
            button {
                onclick: move |e| on_prev.call(e),
                "Prev"
            }

            span {
                "{label}"
            }

            button {
                onclick: move |e| on_next.call(e),
                "Next"
            }
        }
    }
}

#[component]
fn Reader(book: OpenBook) -> Element {
    epub::use_register_asset_handler(book.epub.clone());

    let mut open_book = use_context::<Signal<Option<OpenBook>>>();
    let docs = book.docs;
    let state = nav::use_reader_state(docs.len());
    let chapter = state.data.chapter();
    let pending_fragment = state.data.pending_fragment();
    let (page, page_count) = (state.data.page(), state.data.page_count());
    let docs_for_iframe = docs.clone();
    let iframe_src = use_memo(move || {
        let current_doc = &docs_for_iframe[chapter()];
        epub::render_document_url(current_doc, pending_fragment().as_deref())
    });

    let page_label = format!("Page {} of {}", page() + 1, page_count());
    let chapter_label = format!("Chapter {} of {}", chapter() + 1, state.chapter_count);

    use_effect(move || {
        let page_number = page();
        let script = format!(
            r#"
            const iframe = document.getElementById("reader-frame");
            if (iframe && iframe.contentWindow) {{
                iframe.contentWindow.postMessage(
                    {{ kind: "ook-set-page", page: {} }},
                    "*"
                );
            }}
        "#,
            page_number
        );
        document::eval(&script);
    });

    use_bridge(state, docs);

    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100vh;",
            p {
                style: "text-align: center",
                "{book.title}"
            }

            iframe {
                id: "reader-frame",
                "sandbox": "allow-same-origin allow-scripts",
                style: "flex: 1; width: 100%; border: none;",
                src: "{iframe_src}",
            }

            div {
                style: "position: absolute; top: 8px; left: 8px;",
                button {
                    onclick: move |_| open_book.set(None),

                    "Close"
                }
            }

            NavRow {
                on_prev: move |_| state.chapter_prev(),
                on_next: move |_| state.chapter_next(),
                label: chapter_label,
            }

            NavRow {
                on_prev: move |_| state.page_prev(),
                on_next: move |_| state.page_next(),
                label: page_label,
            }
        }
    }
}

fn open_epub(path: &std::path::Path) -> Result<(Epub, Vec<SpineDoc>), Box<dyn std::error::Error>> {
    let epub = Epub::open(path)?;
    let docs = epub::load_spine(&epub)?;
    Ok((epub, docs))
}

#[component]
fn LibraryBooks() -> Element {
    let library = use_context::<Rc<Library>>();
    let mut books = use_context::<Signal<Vec<library::Book>>>();
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

                        // "{book.title}"
                        // if let Some(author) = book.author.as_deref() {
                        //     span {
                        //         "{author} "
                        //     }
                        // }
                        button {
                            onclick: {
                                let library = Rc::clone(&library);
                                let id = book.id;

                                move |_| {
                                    if library.remove(id).is_ok() {
                                        if let Ok(list) = library.list() {
                                            books.set(list);
                                        }
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
fn ImportControl() -> Element {
    let library = use_context::<Rc<Library>>();
    let mut books = use_context::<Signal<Vec<library::Book>>>();
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
                                if let Ok(list) = library.list() {
                                    books.set(list);
                                }
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

fn use_bridge(state: ReaderState, docs: Rc<Vec<epub::SpineDoc>>) {
    use_future(move || {
        let docs = docs.clone();
        async move {
            let mut bridge = document::eval(BRIDGE_JS);

            while let Ok(msg) = bridge.recv::<String>().await {
                match BridgeMsg::parse(&msg) {
                    Some(BridgeMsg::Link(href)) => {
                        let idx = *state.data.chapter().peek();
                        if let Some(target) = epub::resolve_internal_link(&docs, idx, &href) {
                            state.follow_link(target);
                        }
                    }
                    Some(BridgeMsg::Scroll(page)) => state.on_scroll(page),
                    Some(BridgeMsg::Pages(p_count)) => state.on_pages(p_count),
                    None => {}
                }
            }
        }
    });
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bridge_parses_each_message_kind() {
        assert_eq!(BridgeMsg::parse("scroll:3"), Some(BridgeMsg::Scroll(3)));
        assert_eq!(BridgeMsg::parse("pages:12"), Some(BridgeMsg::Pages(12)));
        assert_eq!(
            BridgeMsg::parse("link:chapter2.xhtml#s3"),
            Some(BridgeMsg::Link("chapter2.xhtml#s3".to_string()))
        );
        // unknown prefixes and malformed numbers decode to None, never panic
        assert_eq!(BridgeMsg::parse("scroll:notanumber"), None);
        assert_eq!(BridgeMsg::parse("bogus:1"), None);
    }
}
