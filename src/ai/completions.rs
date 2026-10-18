use serde::{Deserialize, Serialize};

use super::{
    api_error, http, non_empty_reply, sse::SseBuffer, ChatError, ChatProvider, Message, Progress,
    Reply, Role,
};

const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Debug, Serialize)]
struct CompletionRequest<'a> {
    model: &'a str,
    messages: Vec<CompletionMessage<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct CompletionMessage<'a> {
    role: &'static str,
    content: &'a str,
}

fn request_body<'a>(
    model: &'a str,
    messages: &'a [Message],
    reasoning_effort: Option<&'static str>,
) -> CompletionRequest<'a> {
    CompletionRequest {
        model,
        messages: messages.iter().map(CompletionMessage::from).collect(),
        stream: true,
        reasoning_effort,
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
    refusal: Option<String>,
    reasoning_content: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Thinking,
    Text(String),
    Done,
}

fn event(payload: &str) -> Result<Event, serde_json::Error> {
    if payload == "[DONE]" {
        return Ok(Event::Done);
    }

    let chunk: CompletionChunk = serde_json::from_str(payload)?;
    let delta = chunk
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.delta)
        .unwrap_or_default();
    let text = delta.content.or(delta.refusal).unwrap_or_default();
    let reasoning = delta.reasoning_content.unwrap_or_default();

    if text.is_empty() && !reasoning.is_empty() {
        return Ok(Event::Thinking);
    }

    Ok(Event::Text(text))
}

#[derive(Clone)]
pub(crate) struct Completions {
    base_url: &'static str,
    reasoning_effort: Option<&'static str>,
    key: String,
    model: String,
}

impl Completions {
    fn new(
        base_url: &'static str,
        reasoning_effort: Option<&'static str>,
        key: String,
        model: impl Into<String>,
    ) -> Self {
        Completions {
            base_url,
            reasoning_effort,
            key,
            model: model.into(),
        }
    }

    pub(crate) fn openai(key: String, model: impl Into<String>) -> Self {
        Completions::new(OPENAI_BASE_URL, Some("none"), key, model)
    }

    pub(crate) fn request(&self, messages: &[Message]) -> reqwest::RequestBuilder {
        http()
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.key)
            .json(&request_body(&self.model, messages, self.reasoning_effort))
    }
}

impl ChatProvider for Completions {
    async fn stream(
        &self,
        messages: &[Message],
        mut on_progress: impl FnMut(Progress<'_>),
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
                    Event::Thinking => on_progress(Progress::Thinking),
                    Event::Text(delta) => {
                        on_progress(Progress::Text(&delta));
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

        let body = serde_json::to_value(request_body("gpt-6-luna", &messages, None)).unwrap();

        assert_eq!(
            body,
            json!({
                "model": "gpt-6-luna",
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
            "model": "gpt-6-luna",
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
    fn a_reasoning_chunk_is_thinking() {
        let payload = r#"{ "choices": [ { "index": 0, "delta": { "content": null, "reasoning_content": "The user asks" } } ] }"#;

        assert_eq!(event(payload).unwrap(), Event::Thinking);
    }

    #[test]
    fn a_refusal_chunk_yields_the_refusal_as_text() {
        let payload = r#"{ "choices": [ { "index": 0, "delta": { "content": null, "refusal": "I can't help with that." } } ] }"#;

        assert_eq!(
            event(payload).unwrap(),
            Event::Text("I can't help with that.".into())
        );
    }

    #[test]
    fn the_done_marker_ends_the_stream() {
        assert_eq!(event("[DONE]").unwrap(), Event::Done);
    }

    #[test]
    fn a_payload_that_is_not_json_is_an_error() {
        assert!(event("[DONE").is_err());
    }

    fn body(request: &reqwest::Request) -> serde_json::Value {
        serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap()
    }

    #[test]
    fn an_openai_request_posts_the_conversation_with_a_bearer_key_and_no_reasoning() {
        let openai = Completions::openai("openai-key".to_owned(), "gpt-6-luna");

        let request = openai
            .request(&[Message::user("Which city?")])
            .build()
            .unwrap();

        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(
            request.url().as_str(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(request.headers()["authorization"], "Bearer openai-key");
        assert_eq!(
            body(&request),
            json!({
                "model": "gpt-6-luna",
                "messages": [ { "role": "user", "content": "Which city?" } ],
                "stream": true,
                "reasoning_effort": "none"
            })
        );
    }

    #[tokio::test]
    #[ignore = "needs OPENAI_API_KEY and the network"]
    async fn a_real_luna_streams_through_the_trait() {
        let key = std::env::var("OPENAI_API_KEY").expect("set OPENAI_API_KEY to run this");
        let luna = Completions::openai(key, "gpt-6-luna");
        let messages = [Message::user("Count from one to twenty in words")];
        let mut pieces = Vec::new();

        let reply = ChatProvider::stream(&luna, &messages, |progress| {
            if let Progress::Text(delta) = progress {
                pieces.push(delta.to_owned());
            }
        })
        .await
        .unwrap();

        assert!(pieces.len() > 1, "{pieces:?}");
        assert_eq!(pieces.concat(), reply.text);
    }
}
