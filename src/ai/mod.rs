pub(crate) mod gemini;
pub(crate) mod prompt;
mod sse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Provider {
    Gemini,
    OpenCodeZen,
}

impl Provider {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Provider::Gemini => "Gemini",
            Provider::OpenCodeZen => "OpenCode Zen",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
    role: Role,
    text: String,
}

impl Message {
    pub(crate) fn user(text: impl Into<String>) -> Self {
        Message {
            role: Role::User,
            text: text.into(),
        }
    }

    pub(crate) fn assistant(text: impl Into<String>) -> Self {
        Message {
            role: Role::Assistant,
            text: text.into(),
        }
    }

    pub(crate) fn role(&self) -> Role {
        self.role
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reply {
    pub(crate) text: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ChatError {
    #[error("the provider returned no answer")]
    Empty,
    #[error("could not reach the provider: {0}")]
    Http(#[from] reqwest::Error),
    #[error("the provider rejected the request ({status}): {body}")]
    Api { status: u16, body: String },
    #[error("could not read the provider's answer: {0}")]
    Json(#[from] serde_json::Error),
}

pub(crate) trait ChatProvider {
    async fn stream(
        &self,
        messages: &[Message],
        on_text: impl FnMut(&str),
    ) -> Result<Reply, ChatError>;
}

#[cfg(test)]
mod test {
    use super::*;
    use std::cell::RefCell;

    struct Fake {
        reply: &'static str,
        seen: RefCell<Vec<Message>>,
    }

    impl ChatProvider for Fake {
        async fn stream(
            &self,
            messages: &[Message],
            mut on_text: impl FnMut(&str),
        ) -> Result<Reply, ChatError> {
            self.seen.borrow_mut().extend_from_slice(messages);
            for word in self.reply.split_inclusive(' ') {
                on_text(word);
            }
            Ok(Reply {
                text: self.reply.to_string(),
            })
        }
    }

    #[test]
    fn each_provider_has_a_reader_facing_label() {
        assert_eq!(
            [Provider::Gemini, Provider::OpenCodeZen].map(Provider::label),
            ["Gemini", "OpenCode Zen"]
        );
    }

    #[test]
    fn a_provider_streams_its_answer_in_pieces() {
        let fake = Fake {
            reply: "Ankh-Morpork on the Ankh",
            seen: RefCell::new(Vec::new()),
        };
        let question = Message::user("Which city?");
        let mut pieces = Vec::new();

        let reply = pollster::block_on(fake.stream(std::slice::from_ref(&question), |delta| {
            pieces.push(delta.to_owned())
        }))
        .unwrap();

        assert_eq!(pieces, ["Ankh-Morpork ", "on ", "the ", "Ankh"]);
        assert_eq!(
            pieces.concat(),
            reply.text,
            "the pieces add up to the reply"
        );
        assert_eq!(fake.seen.borrow().as_slice(), &[question]);
    }
}
