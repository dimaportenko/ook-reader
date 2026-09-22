use crate::ai::{ChatError, Message, Reply};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum Status {
    #[default]
    Idle,
    Waiting,
    Failed(String),
}

#[derive(Debug, Default)]
pub(crate) struct Conversation {
    messages: Vec<Message>,
    status: Status,
}

impl Conversation {
    pub(crate) fn ask(&mut self, question: &str) -> bool {
        let question = question.trim();
        if question.is_empty() || self.status == Status::Waiting {
            return false;
        }

        self.messages.push(Message::user(question));
        self.status = Status::Waiting;

        true
    }

    pub(crate) fn settle(&mut self, outcome: Result<Reply, ChatError>) {
        self.status = match outcome {
            Ok(reply) => {
                self.messages.push(Message::assistant(reply.text));
                Status::Idle
            }
            Err(error) => Status::Failed(error.to_string()),
        };
    }

    pub(crate) fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub(crate) fn status(&self) -> &Status {
        &self.status
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ai::Role;

    fn reply(text: &str) -> Result<Reply, ChatError> {
        Ok(Reply {
            text: text.to_owned(),
        })
    }

    #[test]
    fn asking_records_the_turn_and_waits() {
        let mut chat = Conversation::default();

        assert!(chat.ask("Which city?"));

        assert_eq!(chat.messages(), &[Message::user("Which city?")]);
        assert_eq!(chat.status(), &Status::Waiting);
    }

    #[test]
    fn a_blank_question_is_not_a_turn() {
        let mut chat = Conversation::default();

        assert!(!chat.ask("  \n"));

        assert!(chat.messages().is_empty());
        assert_eq!(chat.status(), &Status::Idle);
    }

    #[test]
    fn a_question_is_stored_trimmed() {
        let mut chat = Conversation::default();

        chat.ask("  Which city?\n");

        assert_eq!(chat.messages()[0].text(), "Which city?");
        assert_eq!(chat.messages()[0].role(), Role::User);
    }

    #[test]
    fn a_reply_lands_as_an_assistant_turn() {
        let mut chat = Conversation::default();
        chat.ask("Which city?");

        chat.settle(reply("Ankh-Morpork"));

        assert_eq!(
            chat.messages(),
            &[
                Message::user("Which city?"),
                Message::assistant("Ankh-Morpork")
            ]
        );
        assert_eq!(chat.status(), &Status::Idle);
    }

    #[test]
    fn a_failure_keeps_the_question_so_it_can_be_asked_again() {
        let mut chat = Conversation::default();
        chat.ask("Which city?");

        chat.settle(Err(ChatError::Empty));

        assert_eq!(chat.messages(), &[Message::user("Which city?")]);
        assert_eq!(
            chat.status(),
            &Status::Failed("the provider returned no answer".to_owned())
        );
        assert!(
            chat.ask("Which city, again?"),
            "a failed chat accepts a new turn"
        );
    }

    #[test]
    fn asking_while_waiting_is_refused() {
        let mut chat = Conversation::default();
        chat.ask("First?");

        assert!(!chat.ask("Second?"));

        assert_eq!(chat.messages().len(), 1);
    }
}
