use crate::{
    ai::{gemini::Gemini, opencode::OpenCode, AnyProvider, ChatModel, Provider},
    secrets::{api_key, SecretError, SecretStore},
};

pub(crate) fn provider_for(
    store: &dyn SecretStore,
    model: &ChatModel,
) -> Result<Option<AnyProvider>, SecretError> {
    Ok(
        api_key::get(store, model.provider)?.and_then(|key| match model.provider {
            Provider::Gemini => Some(AnyProvider::Gemini(Gemini::new(key, &model.id))),
            Provider::OpenCodeZen => Some(AnyProvider::OpenCode(OpenCode::new(key, &model.id))),
            Provider::OpenAi => None,
        }),
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{ai::gemini::GeminiModel, secrets::Memory};

    #[test]
    fn the_chosen_model_needs_its_own_provider_s_key() {
        let store = Memory::default();
        let gemini = ChatModel::gemini(GeminiModel::Flash);
        let zen = ChatModel {
            provider: Provider::OpenCodeZen,
            id: "kimi-k2.6".to_owned(),
        };
        let client = |model: &ChatModel| provider_for(&store, model).expect("read the key");

        assert!(client(&gemini).is_none());
        assert!(client(&zen).is_none());

        api_key::save(&store, Provider::OpenCodeZen, "zen").expect("save the Zen key");
        assert!(matches!(client(&zen), Some(AnyProvider::OpenCode(_))));
        assert!(client(&gemini).is_none(), "a Zen key doesn't unlock Gemini");

        api_key::save(&store, Provider::Gemini, "gemini").expect("save the Gemini key");
        assert!(matches!(client(&gemini), Some(AnyProvider::Gemini(_))));

        api_key::forget(&store, Provider::OpenCodeZen).expect("forget the Zen key");
        assert!(client(&zen).is_none());
    }

    #[test]
    fn an_openai_key_builds_no_client_yet() {
        let store = Memory::default();
        let luna = ChatModel {
            provider: Provider::OpenAi,
            id: "gpt-6-luna".to_owned(),
        };
        api_key::save(&store, Provider::OpenAi, "openai").expect("save the OpenAI key");

        assert!(provider_for(&store, &luna).expect("read the key").is_none());
    }
}
