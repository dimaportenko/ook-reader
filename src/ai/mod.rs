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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reply {
    pub(crate) text: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ChatError {
    #[error("the provider returned no answer")]
    Empty,
}

pub(crate) trait ChatProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Reply, ChatError>;
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
        async fn complete(&self, messages: &[Message]) -> Result<Reply, ChatError> {
            self.seen.borrow_mut().extend_from_slice(messages);
            Ok(Reply {
                text: self.reply.to_string(),
            })
        }
    }

    #[test]
    fn a_provider_receives_the_conversation_and_answers() {
        let fake = Fake {
            reply: "Ankh-Morpork",
            seen: RefCell::new(Vec::new()),
        };
        let question = Message::user("Which city?");

        let reply = pollster::block_on(fake.complete(std::slice::from_ref(&question))).unwrap();

        assert_eq!(reply.text, "Ankh-Morpork");
        assert_eq!(fake.seen.borrow().as_slice(), &[question]);
    }
}
