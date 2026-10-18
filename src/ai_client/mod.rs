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
            Provider::OpenCodeZen => AnyProvider::Completions(Completions::zen(key, &model.id)),
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
        let zen = ChatModel {
            provider: Provider::OpenCodeZen,
            id: "kimi-k2.6".to_owned(),
        };
        let client = |model: &ChatModel| provider_for(&store, model).expect("read the key");

        assert!(client(&gemini).is_none());
        assert!(client(&zen).is_none());

        api_key::save(&store, Provider::OpenCodeZen, "zen").expect("save the Zen key");
        assert!(matches!(client(&zen), Some(AnyProvider::Completions(_))));
        assert!(client(&gemini).is_none(), "a Zen key doesn't unlock Gemini");

        api_key::save(&store, Provider::Gemini, "gemini").expect("save the Gemini key");
        assert!(matches!(client(&gemini), Some(AnyProvider::Gemini(_))));

        api_key::forget(&store, Provider::OpenCodeZen).expect("forget the Zen key");
        assert!(client(&zen).is_none());
    }

    #[test]
    fn zen_and_luna_post_to_their_own_hosts() {
        let store = Memory::default();
        let zen = ChatModel {
            provider: Provider::OpenCodeZen,
            id: "kimi-k2.6".to_owned(),
        };
        api_key::save(&store, Provider::OpenCodeZen, "zen").expect("save the Zen key");
        api_key::save(&store, Provider::OpenAi, "openai").expect("save the OpenAI key");
        let url = |model: &ChatModel| match provider_for(&store, model).expect("read the key") {
            Some(AnyProvider::Completions(client)) => client
                .request(&[Message::user("Hi")])
                .build()
                .expect("build the request")
                .url()
                .to_string(),
            _ => panic!("{} builds no chat/completions client", model.key()),
        };

        assert_eq!(url(&zen), "https://opencode.ai/zen/v1/chat/completions");
        assert_eq!(
            url(&ChatModel::luna()),
            "https://api.openai.com/v1/chat/completions"
        );
    }
}
