use serde::Serialize;

use crate::ai::{Message, Role};

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

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_conversation_becomes_gemini_contents() {
        let messages = [Message::user("Which city?"), Message::assistant("Ankh-Morpork")];

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
}
