use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use super::{ChatError, ChatProvider, Message, Reply, Role};

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
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    #[serde(default)]
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

fn reply_from(response: GenerateResponse) -> Result<Reply, ChatError> {
    let Some(candidate) = response.candidates.into_iter().next() else {
        return Err(ChatError::Empty);
    };

    let text: String = candidate
        .content
        .parts
        .into_iter()
        .filter_map(|part| part.text)
        .collect();

    if text.is_empty() {
        Err(ChatError::Empty)
    } else {
        Ok(Reply { text })
    }
}

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
    format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent")
}

fn check_status(status: StatusCode, body: String) -> Result<String, ChatError> {
    if status.is_success() {
        Ok(body)
    } else {
        Err(ChatError::Api {
            status: status.as_u16(),
            body,
        })
    }
}

impl ChatProvider for Gemini {
    async fn complete(&self, messages: &[Message]) -> Result<Reply, ChatError> {
        let response = self
            .client
            .post(endpoint(&self.model))
            .header("x-goog-api-key", &self.key)
            .json(&request_body(messages))
            .send()
            .await?;

        let status = response.status();
        let body = check_status(status, response.text().await?)?;

        let parsed: GenerateResponse = serde_json::from_str(&body)?;
        reply_from(parsed)
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
    fn a_captured_success_becomes_a_reply() {
        let body = r#"{
            "candidates": [
                {
                    "content": { "role": "model", "parts": [ { "text": "Ankh-Morpork" } ] },
                    "finishReason": "STOP"
                }
            ],
            "usageMetadata": { "promptTokenCount": 9, "candidatesTokenCount": 3 }
        }"#;

        let reply = reply_from(serde_json::from_str(body).unwrap()).unwrap();

        assert_eq!(reply.text, "Ankh-Morpork");
    }

    #[test]
    fn several_text_parts_are_joined_into_one_reply() {
        let body = r#"{ "candidates": [ { "content": { "parts": [
            { "text": "Ankh-" }, { "text": "Morpork" }
        ] } } ] }"#;

        let reply = reply_from(serde_json::from_str(body).unwrap()).unwrap();

        assert_eq!(reply.text, "Ankh-Morpork");
    }

    #[test]
    fn a_blocked_prompt_has_no_candidates_and_is_empty() {
        let body = r#"{ "promptFeedback": { "blockReason": "SAFETY" } }"#;

        let result = reply_from(serde_json::from_str(body).unwrap());

        assert!(matches!(result, Err(ChatError::Empty)), "{result:?}");
    }

    #[test]
    fn a_candidate_with_no_text_is_empty_too() {
        let body = r#"{ "candidates": [ { "content": { "parts": [ {} ] } } ] }"#;

        let result = reply_from(serde_json::from_str(body).unwrap());

        assert!(matches!(result, Err(ChatError::Empty)), "{result:?}");
    }

    #[test]
    fn the_endpoint_names_the_model_and_the_method() {
        let url = endpoint("gemini-3.5-flash-lite");

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash-lite:generateContent"
        );
    }

    #[test]
    fn a_chosen_model_reaches_the_endpoint() {
        let gemini = Gemini::new("key".to_owned(), "gemini-3.5-flash");

        assert_eq!(
            endpoint(&gemini.model),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent"
        );
    }

    #[test]
    fn a_non_success_status_becomes_an_api_error() {
        let result = check_status(StatusCode::BAD_REQUEST, r#"{"error":{"code":400}}"#.into());

        assert!(
            matches!(&result, Err(ChatError::Api { status: 400, body }) if body.contains("400")),
            "{result:?}"
        );
    }

    #[tokio::test]
    #[ignore = "needs GEMINI_API_KEY and the network"]
    async fn a_real_gemini_answers_through_the_trait() {
        let key = std::env::var("GEMINI_API_KEY").expect("set GEMINI_API_KEY to run this");
        let gemini = Gemini::new(key, "gemini-3.5-flash-lite");
        let messages = [Message::user("Reply with exactly one word: pong")];

        let reply = ChatProvider::complete(&gemini, &messages).await.unwrap();

        assert!(reply.text.to_lowercase().contains("pong"), "{reply:?}");
    }
}
