use serde::Deserialize;

use super::{ChatError, ChatProvider, Message, Reply};

const MODELS_URL: &str = "https://opencode.ai/zen/v1/models";

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
    let response = reqwest::get(MODELS_URL).await?;

    let status = response.status();
    if !status.is_success() {
        return Err(ChatError::Api {
            status: status.as_u16(),
            body: response.text().await?,
        });
    }

    Ok(model_ids(response.json().await?))
}

#[derive(Debug, Clone)]
pub(crate) struct OpenCode {
    model: String,
}

impl OpenCode {
    pub(crate) fn new(model: impl Into<String>) -> Self {
        OpenCode {
            model: model.into(),
        }
    }
}

impl ChatProvider for OpenCode {
    async fn stream(
        &self,
        _messages: &[Message],
        mut on_text: impl FnMut(&str),
    ) -> Result<Reply, ChatError> {
        let text = format!(
            "A canned reply from {}, until Zen replies are wired up.",
            self.model
        );
        for word in text.split_inclusive(' ') {
            on_text(word);
        }
        Ok(Reply { text })
    }
}

#[cfg(test)]
mod test {
    use super::*;

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

    #[tokio::test]
    #[ignore = "needs the network"]
    async fn the_real_catalog_lists_a_known_chat_model() {
        let ids = models().await.unwrap();

        assert!(ids.iter().any(|id| id == "deepseek-v4-flash"), "{ids:?}");
    }
}
