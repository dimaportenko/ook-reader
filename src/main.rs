#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::desktop::{use_asset_handler, wry::http::Response};
use dioxus::prelude::*;
use rbook::Epub;

mod epub;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

pub(crate) const BOOK: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/book/pg1661-adventures-of-sherlock-holmes.epub"
);

#[derive(Debug, PartialEq)]
enum Seek {
    First,
    Last,
}

#[derive(Debug, PartialEq)]
enum Nav {
    Stay,
    Page(usize),
    Chapter { index: usize, seek: Seek },
}

#[derive(Store, Default)]
struct ReaderData {
    chapter: usize,
    page: usize,
    page_count: usize,
    pending_fragment: Option<String>,
    pending_last: bool,
}

#[derive(Clone, Copy)]
struct ReaderState {
    data: Store<ReaderData>,
    chapter_count: usize,
}

fn use_reader_state(chapter_count: usize) -> ReaderState {
    ReaderState {
        data: use_store(ReaderData::default),
        chapter_count,
    }
}

impl ReaderState {
    fn page_prev(self) {
        let (page, chapter) = (self.data.page(), self.data.chapter());
        self.apply(on_prev(page(), chapter()));
    }

    fn page_next(self) {
        let (page, page_count, chapter) = (
            self.data.page(),
            self.data.page_count(),
            self.data.chapter(),
        );
        self.apply(on_next(page(), page_count(), chapter(), self.chapter_count));
    }

    fn apply(self, nav: Nav) {
        let (mut page, mut chapter) = (self.data.page(), self.data.chapter());
        match nav {
            Nav::Stay => {}
            Nav::Page(p) => page.set(p),
            Nav::Chapter {
                index,
                seek: Seek::First,
            } => {
                page.set(0);
                chapter.set(index);
            }
            Nav::Chapter {
                index,
                seek: Seek::Last,
            } => {
                chapter.set(index);
                self.data.pending_last().set(true);
            }
        }
    }

    fn chapter_prev(self) {
        let mut chapter = self.data.chapter();
        self.data.page().set(0);
        chapter.set(prev_index(chapter()));
    }

    fn chapter_next(self) {
        let mut chapter = self.data.chapter();
        self.data.page().set(0);
        chapter.set(next_index(chapter(), self.chapter_count));
    }

    fn follow_link(self, target: epub::LinkTarget) {
        self.data.chapter().set(target.spine_index);
        self.data.page().set(0);
        self.data.pending_fragment().set(target.fragment);
    }

    fn on_scroll(self, p: usize) {
        self.data.page().set(p);
        self.data.pending_fragment().set(None);
    }

    fn on_pages(self, pages: usize) {
        let (mut page, mut pending_last) = (self.data.page(), self.data.pending_last());
        self.data.page_count().set(pages);
        if pending_last() {
            page.set(pages.saturating_sub(1));
            pending_last.set(false);
        }
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

fn on_next(page: usize, page_count: usize, chapter: usize, chapter_count: usize) -> Nav {
    if page_count > 0 && page + 1 < page_count {
        Nav::Page(page + 1)
    } else if chapter + 1 < chapter_count {
        Nav::Chapter {
            index: chapter + 1,
            seek: Seek::First,
        }
    } else {
        Nav::Stay
    }
}

fn on_prev(page: usize, chapter: usize) -> Nav {
    if page > 0 {
        Nav::Page(page - 1)
    } else if chapter > 0 {
        Nav::Chapter {
            index: chapter - 1,
            seek: Seek::Last,
        }
    } else {
        Nav::Stay
    }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let epub = use_hook(|| Rc::new(Epub::open(BOOK).expect("should open the bundled epub")));

    use_asset_handler(epub::EPUB_ROUTE, move |request, responder| {
        let path = request
            .uri()
            .path()
            .strip_prefix(&format!("/{}", epub::EPUB_ROUTE))
            .unwrap_or_default();

        match epub.read_resource_bytes(path) {
            Ok(bytes) => {
                let body = Response::builder()
                    .header("Content-Type", epub::content_type_for(path))
                    .body(bytes)
                    .expect("response with a valid content-type header");
                responder.respond(body);
            }
            Err(_) => {
                let not_found = Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .expect("empty 404 body is always valid");
                responder.respond(not_found);
            }
        }
    });

    rsx! {
        document::Link {
            rel: "icon",
            href: FAVICON,
        }
        document::Link {
            rel: "stylesheet",
            href: MAIN_CSS,
        }
        Reader {}
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
fn Reader() -> Element {
    let docs = use_hook(|| epub::load_spine(BOOK).expect("bundled epub should load"));
    let state = use_reader_state(docs.len());
    let chapter = state.data.chapter();
    let pending_fragment = state.data.pending_fragment();
    let (page, page_count) = (state.data.page(), state.data.page_count());
    let current_doc = &docs[chapter()];
    let iframe_src = epub::render_document_url(current_doc, page(), pending_fragment().as_deref());

    let page_label = format!("Page {} of {}", page() + 1, page_count());
    let chapter_label = format!("Chapter {} of {}", chapter() + 1, state.chapter_count);

    use_future(move || {
        let docs = docs.clone();
        async move {
            let mut bridge = document::eval(
                r#"
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
            "#,
            );

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

    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100vh;",

            iframe {
                "sandbox": "allow-same-origin allow-scripts",
                style: "flex: 1; width: 100%; border: none;",
                src: "{iframe_src}",
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

fn prev_index(chapter: usize) -> usize {
    chapter.saturating_sub(1)
}

fn next_index(chapter: usize, len: usize) -> usize {
    (chapter + 1).min(len.saturating_sub(1))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn paging_clamps_at_both_ends() {
        let len = 15;

        assert_eq!(next_index(0, len), 1);

        assert_eq!(next_index(len - 1, len), len - 1);

        assert_eq!(prev_index(5), 4);

        assert_eq!(prev_index(0), 0);
    }

    #[test]
    fn page_nav_rolls_over_chapter_boundaries() {
        assert_eq!(on_next(0, 3, 0, 15), Nav::Page(1));
        assert_eq!(on_prev(2, 3), Nav::Page(1));

        assert_eq!(
            on_next(2, 3, 0, 15),
            Nav::Chapter {
                index: 1,
                seek: Seek::First
            }
        );
        assert_eq!(on_next(2, 3, 14, 15), Nav::Stay);

        assert_eq!(
            on_prev(0, 3),
            Nav::Chapter {
                index: 2,
                seek: Seek::Last
            }
        );
        assert_eq!(on_prev(0, 0), Nav::Stay);

        assert_eq!(
            on_next(0, 0, 0, 15),
            Nav::Chapter {
                index: 1,
                seek: Seek::First
            }
        );
    }

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
