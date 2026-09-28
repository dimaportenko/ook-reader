use serde::{Deserialize, Serialize};

use super::{
    api_error, http, non_empty_reply, sse::SseBuffer, ChatError, ChatProvider, Message, Reply, Role,
};

const MODELS_URL: &str = "https://opencode.ai/zen/v1/models";
const COMPLETIONS_URL: &str = "https://opencode.ai/zen/v1/chat/completions";

#[derive(Debug, Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

fn model_ids(list: ModelList) -> Vec<String> {
    list.data.into_iter().map(|entry| entry.id).collect()
}

pub(crate) async fn models() -> Result<Vec<String>, ChatError> {
    let response = http().get(MODELS_URL).send().await?;

    let status = response.status();
    if !status.is_success() {
        return Err(api_error(status, response.text().await?));
    }

    Ok(model_ids(response.json().await?))
}

#[derive(Debug, Serialize)]
struct CompletionRequest<'a> {
    model: &'a str,
    messages: Vec<CompletionMessage<'a>>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct CompletionMessage<'a> {
    role: &'static str,
    content: &'a str,
}

fn request_body<'a>(model: &'a str, messages: &'a [Message]) -> CompletionRequest<'a> {
    CompletionRequest {
        model,
        messages: messages.iter().map(CompletionMessage::from).collect(),
        stream: true,
    }
}

impl<'a> From<&'a Message> for CompletionMessage<'a> {
    fn from(message: &'a Message) -> Self {
        CompletionMessage {
            role: match message.role() {
                Role::User => "user",
                Role::Assistant => "assistant",
            },
            content: message.text(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct CompletionChunk {
    #[serde(default)]
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    #[serde(default)]
    delta: Delta,
}

#[derive(Debug, Default, Deserialize)]
struct Delta {
    content: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Text(String),
    Done,
}

fn event(payload: &str) -> Result<Event, serde_json::Error> {
    if payload == "[DONE]" {
        return Ok(Event::Done);
    }

    let chunk: CompletionChunk = serde_json::from_str(payload)?;
    let text = chunk
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.delta.content)
        .unwrap_or_default();

    Ok(Event::Text(text))
}

#[derive(Clone)]
pub(crate) struct OpenCode {
    key: String,
    model: String,
}

impl OpenCode {
    pub(crate) fn new(key: String, model: impl Into<String>) -> Self {
        OpenCode {
            key,
            model: model.into(),
        }
    }

    fn request(&self, messages: &[Message]) -> reqwest::RequestBuilder {
        http()
            .post(COMPLETIONS_URL)
            .bearer_auth(&self.key)
            .json(&request_body(&self.model, messages))
    }
}

impl ChatProvider for OpenCode {
    async fn stream(
        &self,
        messages: &[Message],
        mut on_text: impl FnMut(&str),
    ) -> Result<Reply, ChatError> {
        let mut response = self.request(messages).send().await?;

        let status = response.status();
        if !status.is_success() {
            return Err(api_error(status, response.text().await?));
        }

        let mut sse = SseBuffer::default();
        let mut text = String::new();
        'stream: while let Some(chunk) = response.chunk().await? {
            for payload in sse.push(&chunk) {
                match event(&payload)? {
                    Event::Text(delta) => {
                        on_text(&delta);
                        text.push_str(&delta);
                    }
                    Event::Done => break 'stream,
                }
            }
        }

        non_empty_reply(text)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_conversation_becomes_chat_completions_messages() {
        let messages = [
            Message::user("Which city?"),
            Message::assistant("Ankh-Morpork"),
        ];

        let body = serde_json::to_value(request_body("kimi-k2.6", &messages)).unwrap();

        assert_eq!(
            body,
            json!({
                "model": "kimi-k2.6",
                "messages": [
                    { "role": "user",      "content": "Which city?" },
                    { "role": "assistant", "content": "Ankh-Morpork" },
                ],
                "stream": true
            })
        );
    }

    #[test]
    fn a_captured_chunk_yields_its_delta_text() {
        let payload = r#"{
            "id": "chatcmpl-1",
            "object": "chat.completion.chunk",
            "created": 1790582083,
            "model": "kimi-k2.6",
            "choices": [
                { "index": 0, "delta": { "content": "Ankh-Morpork" }, "finish_reason": null }
            ]
        }"#;

        assert_eq!(event(payload).unwrap(), Event::Text("Ankh-Morpork".into()));
    }

    #[test]
    fn the_opening_chunk_names_the_role_and_yields_nothing() {
        let payload =
            r#"{ "choices": [ { "index": 0, "delta": { "role": "assistant", "content": "" } } ] }"#;

        assert_eq!(event(payload).unwrap(), Event::Text(String::new()));
    }

    #[test]
    fn a_null_content_yields_nothing() {
        let payload = r#"{ "choices": [ { "index": 0, "delta": { "content": null } } ] }"#;

        assert_eq!(event(payload).unwrap(), Event::Text(String::new()));
    }

    #[test]
    fn the_closing_chunk_has_an_empty_delta_and_yields_nothing() {
        let payload = r#"{ "choices": [ { "index": 0, "delta": {}, "finish_reason": "stop" } ] }"#;

        assert_eq!(event(payload).unwrap(), Event::Text(String::new()));
    }

    #[test]
    fn a_usage_chunk_has_no_choices_and_yields_nothing() {
        let payload =
            r#"{ "choices": [], "usage": { "prompt_tokens": 9, "completion_tokens": 3 } }"#;

        assert_eq!(event(payload).unwrap(), Event::Text(String::new()));
    }

    #[test]
    fn the_done_marker_ends_the_stream() {
        assert_eq!(event("[DONE]").unwrap(), Event::Done);
    }

    #[test]
    fn a_payload_that_is_not_json_is_an_error() {
        assert!(event("[DONE").is_err());
    }

    #[test]
    fn a_captured_model_list_yields_its_ids_in_order() {
        let payload = r#"{
            "object": "list",
            "data": [
                { "id": "deepseek-v4-flash", "object": "model", "created": 1790582083, "owned_by": "opencode" },
                { "id": "glm-5.3-flash", "object": "model", "created": 1790582083, "owned_by": "opencode" },
                { "id": "kimi-k2.6", "object": "model", "created": 1790582083, "owned_by": "opencode" }
            ]
        }"#;

        assert_eq!(
            model_ids(serde_json::from_str(payload).unwrap()),
            ["deepseek-v4-flash", "glm-5.3-flash", "kimi-k2.6"]
        );
    }

    #[test]
    fn a_request_posts_the_conversation_to_zen_with_a_bearer_key() {
        let zen = OpenCode::new("zen-key".to_owned(), "kimi-k2.6");

        let request = zen
            .request(&[Message::user("Which city?")])
            .build()
            .unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();

        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(request.url().as_str(), COMPLETIONS_URL);
        assert_eq!(request.headers()["authorization"], "Bearer zen-key");
        assert_eq!(body["model"], "kimi-k2.6");
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"][0]["content"], "Which city?");
    }

    #[tokio::test]
    #[ignore = "needs OPENCODE_API_KEY and the network"]
    async fn a_real_zen_streams_through_the_trait() {
        let key = std::env::var("OPENCODE_API_KEY").expect("set OPENCODE_API_KEY to run this");
        let zen = OpenCode::new(key, "deepseek-v4-flash");
        let messages = [Message::user("Count from one to twenty in words")];
        let mut pieces = Vec::new();

        let reply = ChatProvider::stream(&zen, &messages, |delta| pieces.push(delta.to_owned()))
            .await
            .unwrap();

        assert!(pieces.len() > 1, "{pieces:?}");
        assert_eq!(pieces.concat(), reply.text);
    }

    #[tokio::test]
    #[ignore = "needs the network"]
    async fn the_real_catalog_lists_a_known_chat_model() {
        let ids = models().await.unwrap();

        assert!(ids.iter().any(|id| id == "deepseek-v4-flash"), "{ids:?}");
    }
}
