use dioxus::prelude::*;

#[component]
pub(crate) fn SlugPicker(
    options: Vec<&'static str>,
    selected: &'static str,
    on_pick: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            select {
                onchange: move |event| on_pick.call(event.data.value()),
                for slug in options {
                    option {
                        key: "{slug}",
                        value: slug,
                        selected: slug == selected,
                        {slug}
                    }
                }
            }
        }
    }
}
