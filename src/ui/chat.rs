use std::collections::BTreeSet;

use dioxus::{core::Task, prelude::*};

use crate::{
    ai::{chat_models, gemini::Gemini, ChatModel, ChatProvider, Provider, Role},
    chat::{Conversation, Status},
    settings::Settings,
    ui::{drawer::Drawer, settings::Styles as SettingsStyles},
};

#[css_module("/src/ui/chat.css")]
struct Styles;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct ChatHandle {
    open: Signal<bool>,
    draft: Signal<String>,
    autosend: Signal<bool>,
}

pub(crate) fn use_chat_handle() -> ChatHandle {
    ChatHandle {
        open: use_signal(|| false),
        draft: use_signal(String::new),
        autosend: use_signal(|| false),
    }
}

impl ChatHandle {
    pub(crate) fn show(mut self) {
        self.open.set(true);
    }

    pub(crate) fn send(mut self, text: String) {
        self.draft.set(text);
        self.autosend.set(true);
        self.open.set(true);
    }
}

#[component]
pub(crate) fn ChatPanel(chat: ChatHandle) -> Element {
    rsx! {
        Drawer {
            open: chat.open,
            label: "Chat",
            ModelPicker {}
            ChatConversation { handle: chat }
        }
    }
}

#[component]
fn ModelPicker() -> Element {
    let mut chosen = use_context::<Signal<ChatModel>>();
    let mut settings = use_context::<Signal<Settings>>();
    let zen_ticked = use_context::<Signal<BTreeSet<String>>>();
    let models = chat_models(&zen_ticked.read());

    rsx! {
        select {
            class: "{SettingsStyles::pill_button} {SettingsStyles::choice} {Styles::chat_panel__model}",
            aria_label: "Model",
            onpointerdown: move |e| e.stop_propagation(),
            onchange: move |event| {
                let Some(model) = chat_models(&zen_ticked.read())
                    .into_iter()
                    .find(|model| model.key() == event.value())
                else {
                    return;
                };
                if let Some(ai_model) = model.gemini_model() {
                    settings.write().ai_model = ai_model;
                }
                chosen.set(model);
            },
            for provider in [Provider::Gemini, Provider::OpenCodeZen] {
                if models.iter().any(|model| model.provider == provider) {
                    optgroup {
                        label: provider.label(),
                        for model in models.iter().filter(|model| model.provider == provider) {
                            option {
                                key: "{model.key()}",
                                value: model.key(),
                                selected: *model == *chosen.read(),
                                "{model.id}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ChatConversation(handle: ChatHandle) -> Element {
    let ChatHandle {
        open,
        mut draft,
        mut autosend,
    } = handle;
    let provider = use_context::<Signal<Option<Gemini>>>();
    let chosen = use_context::<Signal<ChatModel>>();
    let mut chat = use_signal(Conversation::default);
    let mut pending_task = use_signal(|| None::<Task>);

    let mut submit = move || {
        if chosen.read().provider != Provider::Gemini {
            return;
        }
        let Some(gemini) = provider.read().clone() else {
            return;
        };

        if !chat.write().ask(&draft.read()) {
            return;
        }
        draft.set(String::new());

        let history = chat.read().messages().to_vec();
        pending_task.set(Some(spawn(async move {
            let outcome = gemini
                .stream(&history, |delta| chat.write().append(delta))
                .await;
            chat.write().settle(outcome);
            pending_task.set(None);
        })));
    };

    let mut stop = move || {
        if let Some(task) = pending_task.take() {
            task.cancel();
        }
        chat.write().stop();
    };

    let mut reset = move || {
        stop();
        chat.set(Conversation::default());
    };

    use_effect(move || {
        if !open() {
            reset();
        }
    });

    use_effect(move || {
        if autosend() {
            autosend.set(false);
            submit();
        }
    });

    let provider = provider.read();
    let conversation = chat.read();

    rsx! {
        if chosen.read().provider == Provider::OpenCodeZen {
            p {
                class: "{Styles::chat_panel__note}",
                "OpenCode Zen replies aren't available yet."
            }
        } else if provider.is_none() {
            p {
                class: "{Styles::chat_panel__note}",
                "Add a Gemini key in settings to chat."
            }
        } else {
            if conversation.messages().is_empty() {
                p {
                    class: "{Styles::chat_panel__note}",
                    "No messages yet."
                }
            }
            ul {
                class: "{Styles::chat_panel__messages}",
                aria_live: "polite",
                for message in conversation.messages().iter() {
                    li {
                        class: "{Styles::chat_panel__turn}",
                        "data-role": if message.role() == Role::User { "user" } else { "assistant" },
                        "{message.text()}"
                    }
                }
                if let Status::Replying(text) = conversation.status() {
                    li {
                        class: "{Styles::chat_panel__turn}",
                        "data-role": "assistant",
                        if text.is_empty() { "..." } else { "{text}" }
                    }
                }
            }
            if let Status::Failed(text) = conversation.status() {
                p {
                    class: "{Styles::chat_panel__error}",
                    role: "alert",
                    "{text}"
                }
            }
            div {
                class: "{Styles::chat_panel__compose}",
                onpointerdown: move |e| e.stop_propagation(),
                textarea {
                    rows: 6,
                    value: "{draft}",
                    placeholder: "Ask about this book",
                    oninput: move |e| draft.set(e.data.value()),
                    onkeydown: move |e| {
                        if sends(&e.key(), e.modifiers().shift(), e.is_composing()) {
                            e.prevent_default();
                            submit();
                        }
                    },
                }
                div {
                    class: "{Styles::chat_panel__compose_actions}",
                    button {
                        class: "{SettingsStyles::pill_button}",
                        disabled: conversation.messages().is_empty() || *conversation.status() != Status::Idle,
                        onclick: move |_| reset(),
                        "Reset"
                    }
                    if matches!(conversation.status(), Status::Replying(_)) {
                        button {
                            class: "{SettingsStyles::pill_button}",
                            onclick: move |_| stop(),
                            "Stop"
                        }
                    } else {
                        button {
                            class: "{SettingsStyles::pill_button}",
                            disabled: draft.read().trim().is_empty(),
                            onclick: move |_| submit(),
                            "Send"
                        }
                    }
                }
            }
        }
    }
}

fn sends(key: &Key, shift: bool, composing: bool) -> bool {
    *key == Key::Enter && !shift && !composing
}

#[cfg(test)]
mod test {
    use dioxus::prelude::Key;

    use super::sends;

    #[test]
    fn enter_sends_but_shift_enter_breaks_the_line() {
        assert!(sends(&Key::Enter, false, false));
        assert!(!sends(&Key::Enter, true, false), "Shift+Enter is a newline");
    }

    #[test]
    fn enter_that_confirms_an_ime_composition_does_not_send() {
        assert!(
            !sends(&Key::Enter, false, true),
            "the Enter that picks a kana candidate would otherwise send half a word",
        );
    }

    #[test]
    fn only_enter_sends() {
        assert!(!sends(&Key::Character("a".into()), false, false));
        assert!(!sends(&Key::Tab, false, false));
    }
}
