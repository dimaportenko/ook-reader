use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use super::{sse::SseBuffer, ChatError, ChatProvider, Message, Reply, Role};

#[derive(Debug, Serialize)]
struct GenerateRequest<'a> {
    contents: Vec<Content<'a>>,
}

#[derive(Debug, Serialize)]
struct Content<'a> {
    role: &'static str,
    parts: Vec<Part<'a>>,
}

#[derive(Debug, Serialize)]
struct Part<'a> {
    text: &'a str,
}

fn request_body(messages: &[Message]) -> GenerateRequest<'_> {
    GenerateRequest {
        contents: messages.iter().map(Content::from).collect(),
    }
}

impl<'a> From<&'a Message> for Content<'a> {
    fn from(message: &'a Message) -> Self {
        Content {
            role: match message.role {
                Role::User => "user",
                Role::Assistant => "model",
            },
            parts: vec![Part {
                text: &message.text,
            }],
        }
    }
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    #[serde(default)]
    content: ResponseContent,
}

#[derive(Debug, Default, Deserialize)]
struct ResponseContent {
    #[serde(default)]
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

fn text_of(response: GenerateResponse) -> String {
    response
        .candidates
        .into_iter()
        .next()
        .map(|candidate| {
            candidate
                .content
                .parts
                .into_iter()
                .filter_map(|part| part.text)
                .collect()
        })
        .unwrap_or_default()
}

fn non_empty_reply(text: String) -> Result<Reply, ChatError> {
    if text.is_empty() {
        Err(ChatError::Empty)
    } else {
        Ok(Reply { text })
    }
}

#[derive(Clone)]
pub(crate) struct Gemini {
    key: String,
    model: String,
    client: reqwest::Client,
}

impl Gemini {
    pub(crate) fn new(key: String, model: impl Into<String>) -> Self {
        Gemini {
            key,
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }
}

fn endpoint(model: &str) -> String {
    format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent?alt=sse")
}

fn api_error(status: StatusCode, body: String) -> ChatError {
    ChatError::Api {
        status: status.as_u16(),
        body,
    }
}

impl ChatProvider for Gemini {
    async fn stream(
        &self,
        messages: &[Message],
        mut on_text: impl FnMut(&str),
    ) -> Result<Reply, ChatError> {
        let mut response = self
            .client
            .post(endpoint(&self.model))
            .header("x-goog-api-key", &self.key)
            .json(&request_body(messages))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(api_error(status, response.text().await?));
        }

        let mut sse = SseBuffer::default();
        let mut text = String::new();
        while let Some(chunk) = response.chunk().await? {
            for payload in sse.push(&chunk) {
                let delta = text_of(serde_json::from_str(&payload)?);
                on_text(&delta);
                text.push_str(&delta);
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
    fn the_conversation_becomes_gemini_contents() {
        let messages = [
            Message::user("Which city?"),
            Message::assistant("Ankh-Morpork"),
        ];

        let body = serde_json::to_value(request_body(&messages)).unwrap();

        assert_eq!(
            body,
            json!({
                "contents": [
                    { "role": "user",  "parts": [{ "text": "Which city?" }] },
                    { "role": "model", "parts": [{ "text": "Ankh-Morpork" }] },
                ]
            })
        );
    }

    #[test]
    fn an_assistant_turn_is_named_model_on_the_wire() {
        let body = serde_json::to_string(&request_body(&[Message::assistant("hi")])).unwrap();

        assert!(body.contains(r#""role":"model""#), "{body}");
        assert!(!body.contains("assistant"), "{body}");
    }

    #[test]
    fn a_captured_chunk_yields_its_text() {
        let payload = r#"{
            "candidates": [
                {
                    "content": { "role": "model", "parts": [ { "text": "Ankh-Morpork" } ] },
                    "finishReason": "STOP"
                }
            ],
            "usageMetadata": { "promptTokenCount": 9, "candidatesTokenCount": 3 }
        }"#;

        assert_eq!(
            text_of(serde_json::from_str(payload).unwrap()),
            "Ankh-Morpork"
        );
    }

    #[test]
    fn several_text_parts_are_joined() {
        let payload = r#"{ "candidates": [ { "content": { "parts": [
            { "text": "Ankh-" }, { "text": "Morpork" }
        ] } } ] }"#;

        assert_eq!(
            text_of(serde_json::from_str(payload).unwrap()),
            "Ankh-Morpork"
        );
    }

    #[test]
    fn a_closing_chunk_with_no_text_yields_nothing() {
        let payload = r#"{ "candidates": [ { "content": { "role": "model", "parts": [
            { "text": "" }
        ] }, "finishReason": "STOP" } ], "usageMetadata": { "promptTokenCount": 9 } }"#;

        assert_eq!(text_of(serde_json::from_str(payload).unwrap()), "");
    }

    #[test]
    fn a_blocked_prompt_has_no_candidates_and_yields_nothing() {
        let payload = r#"{ "promptFeedback": { "blockReason": "SAFETY" } }"#;

        assert_eq!(text_of(serde_json::from_str(payload).unwrap()), "");
    }

    #[test]
    fn a_candidate_with_no_text_part_yields_nothing() {
        let payload = r#"{ "candidates": [ { "content": { "parts": [ {} ] } } ] }"#;

        assert_eq!(text_of(serde_json::from_str(payload).unwrap()), "");
    }

    #[test]
    fn a_candidate_stopped_for_safety_has_no_content_and_yields_nothing() {
        let payload = r#"{ "candidates": [ { "finishReason": "SAFETY" } ] }"#;

        assert_eq!(text_of(serde_json::from_str(payload).unwrap()), "");
    }

    #[test]
    fn a_stream_with_no_text_is_an_empty_reply() {
        let result = non_empty_reply(String::new());

        assert!(matches!(result, Err(ChatError::Empty)), "{result:?}");
    }

    #[test]
    fn the_endpoint_asks_for_server_sent_events() {
        assert_eq!(
            endpoint("gemini-3.5-flash-lite"),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash-lite:streamGenerateContent?alt=sse"
        );
    }

    #[test]
    fn a_chosen_model_reaches_the_endpoint() {
        let gemini = Gemini::new("key".to_owned(), "gemini-3.5-flash");

        assert_eq!(
            endpoint(&gemini.model),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:streamGenerateContent?alt=sse"
        );
    }

    #[test]
    fn a_non_success_status_becomes_an_api_error() {
        let error = api_error(StatusCode::BAD_REQUEST, r#"{"error":{"code":400}}"#.into());

        assert!(
            matches!(&error, ChatError::Api { status: 400, body } if body.contains("400")),
            "{error:?}"
        );
    }

    #[tokio::test]
    #[ignore = "needs GEMINI_API_KEY and the network"]
    async fn a_real_gemini_streams_through_the_trait() {
        let key = std::env::var("GEMINI_API_KEY").expect("set GEMINI_API_KEY to run this");
        let gemini = Gemini::new(key, "gemini-3.5-flash-lite");
        let messages = [Message::user("Count from one to twenty in words")];
        let mut pieces = Vec::new();

        let reply = ChatProvider::stream(&gemini, &messages, |delta| pieces.push(delta.to_owned()))
            .await
            .unwrap();

        assert!(pieces.len() > 1, "{pieces:?}");
        assert_eq!(pieces.concat(), reply.text);
    }
}
