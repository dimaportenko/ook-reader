#![allow(non_snake_case)]
use dioxus::prelude::*;
const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
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
        document::Link {
            rel: "stylesheet",
            href: MAIN_CSS,
        }
        Counter {

        }
    }
}
#[component]
fn Counter() -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        div {

            h1 {

                "Count: {count}"
            }
            button {
                onclick: move |_| *count.write() += 1,
                "Increment"
            }
        }
    }
}
