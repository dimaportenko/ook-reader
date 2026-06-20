#![allow(non_snake_case)]

use dioxus::prelude::*;
use rbook::Epub;

const FAVICON: Asset = asset!("/assets/favicon.ico");
// const MAIN_CSS: Asset = asset!("/assets/main.css");
const BOOK: &str = "book/The Adventures of Sherlock Holmes by Arthur Conan Doyle.epub";

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link {
            rel: "icon",
            href: FAVICON,
        }
        // document::Link {
        //     rel: "stylesheet",
        //     href: MAIN_CSS,
        // }
        SpineList {}
    }
}

#[component]
fn SpineList() -> Element {
    let docs = use_hook(|| load_spine(BOOK).expect("bundled epub should load"));

    rsx! {
        div {
            for (i, doc) in docs.iter().enumerate() {
                div {
                    key: "{i}",
                    dangerous_inner_html: "{doc}",
                }
            }
        }
    }
}

fn load_spine(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let epub = Epub::open(path)?;

    let mut docs = Vec::new();
    for entry in epub.reader() {
        let data = entry?;
        docs.push(data.content().to_string());
    }

    Ok(docs)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn loads_spine_in_reading_order() {
        let docs = load_spine(BOOK).expect("should open the bundled epub");
        assert_eq!(docs.len(), 15);

        assert!(
            docs.iter().any(|d| d.contains("A Scandal in Bohemia")),
            "expected the first story's text somewhere in the spine",
        );

        assert!(
            !docs[0].contains("A Scandal in Bohemia"),
            "index 0 should be the cover, not story one"
        );
    }
}
