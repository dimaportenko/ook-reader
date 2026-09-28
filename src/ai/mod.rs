pub(crate) mod gemini;
pub(crate) mod opencode;
pub(crate) mod prompt;
mod sse;

use std::{collections::BTreeSet, sync::LazyLock};

use reqwest::StatusCode;

use gemini::{Gemini, GeminiModel};
use opencode::OpenCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Provider {
    Gemini,
    OpenCodeZen,
}

impl Provider {
    pub(crate) const ALL: [Provider; 2] = [Provider::Gemini, Provider::OpenCodeZen];

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

    pub(crate) fn from_slug(slug: &str) -> Option<Provider> {
        Provider::ALL
            .into_iter()
            .find(|provider| provider.slug() == slug)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatModel {
    pub(crate) provider: Provider,
    pub(crate) id: String,
}

impl Default for ChatModel {
    fn default() -> Self {
        ChatModel::gemini(GeminiModel::default())
    }
}

impl ChatModel {
    pub(crate) fn gemini(model: GeminiModel) -> Self {
        ChatModel {
            provider: Provider::Gemini,
            id: model.api_name().to_owned(),
        }
    }

    pub(crate) fn key(&self) -> String {
        format!("{}/{}", self.provider.slug(), self.id)
    }
}

pub(crate) fn chat_models(zen_ticked: &BTreeSet<String>) -> Vec<ChatModel> {
    let gemini = GeminiModel::ALL.into_iter().map(ChatModel::gemini);
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

fn http() -> &'static reqwest::Client {
    static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);
    &CLIENT
}

fn api_error(status: StatusCode, body: String) -> ChatError {
    ChatError::Api {
        status: status.as_u16(),
        body,
    }
}

fn non_empty_reply(text: String) -> Result<Reply, ChatError> {
    if text.is_empty() {
        Err(ChatError::Empty)
    } else {
        Ok(Reply { text })
    }
}

pub(crate) trait ChatProvider {
    async fn stream(
        &self,
        messages: &[Message],
        on_text: impl FnMut(&str),
    ) -> Result<Reply, ChatError>;
}

#[derive(Clone)]
pub(crate) enum AnyProvider {
    Gemini(Gemini),
    OpenCode(OpenCode),
}

impl ChatProvider for AnyProvider {
    async fn stream(
        &self,
        messages: &[Message],
        on_text: impl FnMut(&str),
    ) -> Result<Reply, ChatError> {
        match self {
            AnyProvider::Gemini(gemini) => gemini.stream(messages, on_text).await,
            AnyProvider::OpenCode(zen) => zen.stream(messages, on_text).await,
        }
    }
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
                ChatModel::gemini(GeminiModel::FlashLite),
                ChatModel::gemini(GeminiModel::Flash),
                zen("deepseek-v4-flash"),
                zen("kimi-k2.6"),
            ]
        );
    }

    #[test]
    fn a_gemini_chat_model_uses_the_api_name() {
        assert_eq!(
            ChatModel::gemini(GeminiModel::Flash),
            ChatModel {
                provider: Provider::Gemini,
                id: "gemini-3.5-flash".to_owned(),
            }
        );
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
    fn a_provider_slug_reads_back_and_an_unknown_one_does_not() {
        for provider in Provider::ALL {
            assert_eq!(Provider::from_slug(provider.slug()), Some(provider));
        }
        assert_eq!(Provider::from_slug("openai"), None);
    }

    #[test]
    fn a_stream_with_no_text_is_an_empty_reply() {
        let result = non_empty_reply(String::new());

        assert!(matches!(result, Err(ChatError::Empty)), "{result:?}");
    }

    #[test]
    fn a_non_success_status_becomes_an_api_error() {
        let error = api_error(StatusCode::BAD_REQUEST, r#"{"error":{"code":400}}"#.into());

        assert!(
            matches!(&error, ChatError::Api { status: 400, body } if body.contains("400")),
            "{error:?}"
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
