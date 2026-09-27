use dioxus::prelude::*;

#[css_module("/src/ui/components/picker.css")]
struct Styles;

#[component]
pub(crate) fn SlugPicker(
    label: &'static str,
    options: Vec<(&'static str, &'static str)>,
    selected: &'static str,
    on_pick: EventHandler<String>,
) -> Element {
    rsx! {
        select {
            class: "{Styles::slug_picker}",
            aria_label: label,
            onchange: move |event| on_pick.call(event.data.value()),
            for (slug, name) in options {
                option {
                    key: "{slug}",
                    value: slug,
                    selected: slug == selected,
                    {name}
                }
            }
        }
    }
}
