use crate::{
    ai::{completions::Completions, gemini::Gemini, AnyProvider, ChatModel, Provider},
    secrets::{api_key, SecretError, SecretStore},
};

pub(crate) fn provider_for(
    store: &dyn SecretStore,
    model: &ChatModel,
) -> Result<Option<AnyProvider>, SecretError> {
    Ok(
        api_key::get(store, model.provider)?.map(|key| match model.provider {
            Provider::Gemini => AnyProvider::Gemini(Gemini::new(key, &model.id)),
            Provider::OpenAi => AnyProvider::Completions(Completions::openai(key, &model.id)),
        }),
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        ai::{gemini::GeminiModel, Message},
        secrets::Memory,
    };

    #[test]
    fn the_chosen_model_needs_its_own_provider_s_key() {
        let store = Memory::default();
        let gemini = ChatModel::gemini(GeminiModel::Flash);
        let luna = ChatModel::luna();
        let client = |model: &ChatModel| provider_for(&store, model).expect("read the key");

        assert!(client(&gemini).is_none());
        assert!(client(&luna).is_none());

        api_key::save(&store, Provider::OpenAi, "openai").expect("save the OpenAI key");
        assert!(matches!(client(&luna), Some(AnyProvider::Completions(_))));
        assert!(
            client(&gemini).is_none(),
            "an OpenAI key doesn't unlock Gemini"
        );

        api_key::save(&store, Provider::Gemini, "gemini").expect("save the Gemini key");
        assert!(matches!(client(&gemini), Some(AnyProvider::Gemini(_))));

        api_key::forget(&store, Provider::OpenAi).expect("forget the OpenAI key");
        assert!(client(&luna).is_none());
        assert!(client(&gemini).is_some(), "forgetting OpenAI keeps Gemini");
    }

    #[test]
    fn luna_posts_to_openai_s_host() {
        let store = Memory::default();
        api_key::save(&store, Provider::OpenAi, "openai").expect("save the OpenAI key");

        let url = match provider_for(&store, &ChatModel::luna()).expect("read the key") {
            Some(AnyProvider::Completions(client)) => client
                .request(&[Message::user("Hi")])
                .build()
                .expect("build the request")
                .url()
                .to_string(),
            _ => panic!("Luna builds no chat/completions client"),
        };

        assert_eq!(url, "https://api.openai.com/v1/chat/completions");
    }
}
