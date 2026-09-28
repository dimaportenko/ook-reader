pub(crate) mod gemini;
pub(crate) mod opencode;
pub(crate) mod prompt;
mod sse;

use std::collections::BTreeSet;

use crate::settings::ai_model::AiModel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Provider {
    Gemini,
    OpenCodeZen,
}

impl Provider {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Provider::Gemini => "Gemini",
            Provider::OpenCodeZen => "OpenCode Zen",
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Provider::Gemini => "gemini",
            Provider::OpenCodeZen => "opencode-zen",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatModel {
    pub(crate) provider: Provider,
    pub(crate) id: String,
}

impl ChatModel {
    pub(crate) fn gemini(model: AiModel) -> Self {
        ChatModel {
            provider: Provider::Gemini,
            id: model.api_name().to_owned(),
        }
    }

    pub(crate) fn gemini_model(&self) -> Option<AiModel> {
        if self.provider != Provider::Gemini {
            return None;
        }
        AiModel::ALL
            .into_iter()
            .find(|model| model.api_name() == self.id)
    }

    pub(crate) fn key(&self) -> String {
        format!("{}/{}", self.provider.slug(), self.id)
    }
}

pub(crate) fn chat_models(zen_ticked: &BTreeSet<String>) -> Vec<ChatModel> {
    let gemini = AiModel::ALL.into_iter().map(ChatModel::gemini);
    let zen = zen_ticked.iter().map(|id| ChatModel {
        provider: Provider::OpenCodeZen,
        id: id.clone(),
    });

    gemini.chain(zen).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
    role: Role,
    text: String,
}

impl Message {
    pub(crate) fn user(text: impl Into<String>) -> Self {
        Message {
            role: Role::User,
            text: text.into(),
        }
    }

    pub(crate) fn assistant(text: impl Into<String>) -> Self {
        Message {
            role: Role::Assistant,
            text: text.into(),
        }
    }

    pub(crate) fn role(&self) -> Role {
        self.role
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reply {
    pub(crate) text: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ChatError {
    #[error("the provider returned no answer")]
    Empty,
    #[error("could not reach the provider: {0}")]
    Http(#[from] reqwest::Error),
    #[error("the provider rejected the request ({status}): {body}")]
    Api { status: u16, body: String },
    #[error("could not read the provider's answer: {0}")]
    Json(#[from] serde_json::Error),
}

pub(crate) trait ChatProvider {
    async fn stream(
        &self,
        messages: &[Message],
        on_text: impl FnMut(&str),
    ) -> Result<Reply, ChatError>;
}

#[cfg(test)]
mod test {
    use super::*;
    use std::cell::RefCell;

    struct Fake {
        reply: &'static str,
        seen: RefCell<Vec<Message>>,
    }

    impl ChatProvider for Fake {
        async fn stream(
            &self,
            messages: &[Message],
            mut on_text: impl FnMut(&str),
        ) -> Result<Reply, ChatError> {
            self.seen.borrow_mut().extend_from_slice(messages);
            for word in self.reply.split_inclusive(' ') {
                on_text(word);
            }
            Ok(Reply {
                text: self.reply.to_string(),
            })
        }
    }

    #[test]
    fn each_provider_has_a_reader_facing_label() {
        assert_eq!(
            [Provider::Gemini, Provider::OpenCodeZen].map(Provider::label),
            ["Gemini", "OpenCode Zen"]
        );
    }

    #[test]
    fn the_chat_offers_every_gemini_model_then_the_ticked_zen_ones() {
        let ticked = BTreeSet::from(["kimi-k2.6".to_owned(), "deepseek-v4-flash".to_owned()]);
        let zen = |id: &str| ChatModel {
            provider: Provider::OpenCodeZen,
            id: id.to_owned(),
        };

        assert_eq!(
            chat_models(&ticked),
            [
                ChatModel::gemini(AiModel::FlashLite),
                ChatModel::gemini(AiModel::Flash),
                zen("deepseek-v4-flash"),
                zen("kimi-k2.6"),
            ]
        );
    }

    #[test]
    fn a_gemini_chat_model_uses_the_api_name() {
        assert_eq!(
            ChatModel::gemini(AiModel::Flash),
            ChatModel {
                provider: Provider::Gemini,
                id: "gemini-3.5-flash".to_owned(),
            }
        );
    }

    #[test]
    fn only_a_gemini_chat_model_maps_back_to_a_gemini_setting() {
        assert_eq!(
            ChatModel::gemini(AiModel::Flash).gemini_model(),
            Some(AiModel::Flash)
        );

        let zen_with_a_gemini_id = ChatModel {
            provider: Provider::OpenCodeZen,
            id: AiModel::Flash.api_name().to_owned(),
        };
        assert_eq!(zen_with_a_gemini_id.gemini_model(), None);
    }

    #[test]
    fn a_chat_model_key_names_its_provider_and_id() {
        let model = ChatModel {
            provider: Provider::OpenCodeZen,
            id: "kimi-k2.6".to_owned(),
        };

        assert_eq!(model.key(), "opencode-zen/kimi-k2.6");
    }

    #[test]
    fn each_provider_has_a_stable_storage_slug() {
        assert_eq!(
            [Provider::Gemini, Provider::OpenCodeZen].map(Provider::slug),
            ["gemini", "opencode-zen"]
        );
    }

    #[test]
    fn a_provider_streams_its_answer_in_pieces() {
        let fake = Fake {
            reply: "Ankh-Morpork on the Ankh",
            seen: RefCell::new(Vec::new()),
        };
        let question = Message::user("Which city?");
        let mut pieces = Vec::new();

        let reply = pollster::block_on(fake.stream(std::slice::from_ref(&question), |delta| {
            pieces.push(delta.to_owned())
        }))
        .unwrap();

        assert_eq!(pieces, ["Ankh-Morpork ", "on ", "the ", "Ankh"]);
        assert_eq!(
            pieces.concat(),
            reply.text,
            "the pieces add up to the reply"
        );
        assert_eq!(fake.seen.borrow().as_slice(), &[question]);
    }
}
