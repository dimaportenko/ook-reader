use crate::ai::{ChatError, Message, Reply};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum Status {
    #[default]
    Idle,
    Replying(String),
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
        if question.is_empty() || matches!(self.status, Status::Replying(_)) {
            return false;
        }

        self.messages.push(Message::user(question));
        self.status = Status::Replying(String::new());

        true
    }

    pub(crate) fn append(&mut self, delta: &str) {
        if let Status::Replying(text) = &mut self.status {
            text.push_str(delta);
        }
    }

    pub(crate) fn stop(&mut self) {
        let Status::Replying(text) = &mut self.status else {
            return;
        };
        let text = std::mem::take(text);
        self.status = Status::Idle;

        if !text.is_empty() {
            self.messages.push(Message::assistant(text));
        }
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
        assert_eq!(chat.status(), &Status::Replying(String::new()));
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
    fn deltas_grow_the_reply_in_progress() {
        let mut chat = Conversation::default();
        chat.ask("Which city?");

        chat.append("Ankh-");
        chat.append("Morpork");

        assert_eq!(chat.status(), &Status::Replying("Ankh-Morpork".to_owned()));
        assert_eq!(
            chat.messages(),
            &[Message::user("Which city?")],
            "the partial is not a finished turn yet"
        );
    }

    #[test]
    fn settling_a_streamed_reply_makes_one_assistant_turn() {
        let mut chat = Conversation::default();
        chat.ask("Which city?");
        chat.append("Ankh-Morpork");

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
    fn a_delta_with_no_reply_in_progress_is_ignored() {
        let mut chat = Conversation::default();

        chat.append("stray");

        assert_eq!(chat.status(), &Status::Idle);
    }

    #[test]
    fn asking_while_replying_is_refused() {
        let mut chat = Conversation::default();
        chat.ask("First?");
        chat.append("Half an ans");

        assert!(!chat.ask("Second?"));

        assert_eq!(chat.messages().len(), 1);
    }

    #[test]
    fn stopping_keeps_the_partial_as_an_assistant_turn() {
        let mut chat = Conversation::default();
        chat.ask("Which city?");
        chat.append("Ankh-");

        chat.stop();

        assert_eq!(
            chat.messages(),
            &[Message::user("Which city?"), Message::assistant("Ankh-")]
        );
        assert_eq!(chat.status(), &Status::Idle);
        assert!(
            chat.ask("And the river?"),
            "a stopped chat accepts a new turn"
        );
    }

    #[test]
    fn stopping_before_the_first_word_keeps_only_the_question() {
        let mut chat = Conversation::default();
        chat.ask("Which city?");

        chat.stop();

        assert_eq!(chat.messages(), &[Message::user("Which city?")]);
        assert_eq!(chat.status(), &Status::Idle);
    }

    #[test]
    fn stopping_with_no_reply_in_progress_changes_nothing() {
        let mut chat = Conversation::default();
        chat.ask("Which city?");
        chat.settle(Err(ChatError::Empty));

        chat.stop();

        assert_eq!(chat.messages(), &[Message::user("Which city?")]);
        assert_eq!(
            chat.status(),
            &Status::Failed("the provider returned no answer".to_owned())
        );
    }
}
