use serde::{Deserialize, Serialize};

use crate::ai::{ChatError, Message, Reply, Role};

#[derive(Debug, Serialize)]
pub(crate) struct GenerateRequest {
    contents: Vec<Content>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Content {
    role: &'static str,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Part {
    text: String,
}

pub(crate) fn request_body(messages: &[Message]) -> GenerateRequest {
    GenerateRequest {
        contents: messages.iter().map(Content::from).collect(),
    }
}

impl From<&Message> for Content {
    fn from(message: &Message) -> Self {
        Content {
            role: match message.role {
                Role::User => "user",
                Role::Assistant => "model",
            },
            parts: vec![Part {
                text: message.text.clone(),
            }],
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct GenerateResponse {
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

pub(crate) fn reply_from(response: GenerateResponse) -> Result<Reply, ChatError> {
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
}
