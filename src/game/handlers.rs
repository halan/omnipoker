use actix::prelude::*;
use log::info;

use crate::{
    game::{vote::Vote, Game},
    messages::{Broadcast, Connect, Disconnect, Print, Voting},
};

impl Handler<Connect> for Game {
    type Result = ();

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) {
        self.add_user(msg.id, msg.nickname.clone(), msg.addr);

        self.handle(
            Broadcast {
                message: self.users_summary(),
            },
            ctx,
        );
        info!("User connected: {}", msg.nickname);
    }
}

impl Handler<Disconnect> for Game {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, ctx: &mut Context<Self>) {
        let nickname = self.users.get(&msg.id).unwrap().nickname.clone();
        self.remove_user(msg.id);
        self.handle(
            Broadcast {
                message: self.users_summary(),
            },
            ctx,
        );
        info!("User disconnected: {}", nickname);
    }
}

impl Handler<Voting> for Game {
    type Result = ();

    fn handle(&mut self, msg: Voting, ctx: &mut Context<Self>) {
        let vote = Vote::from(msg.vote_text.as_str());

        if let Some(user) = self.users.get(&msg.id) {
            self.votes.insert(msg.id.clone(), vote.clone());

            if let Vote::Option(vote_value) = vote {
                user.session_addr
                    .do_send(Print {
                        message: format!("You voted: {}", vote_value),
                    })
                    .unwrap();
            }
        }

        self.handle(
            Broadcast {
                message: self.votes_summary(),
            },
            ctx,
        );

        if self.all_voted() {
            self.reset_votes();
        }
    }
}

impl Handler<Broadcast> for Game {
    type Result = ();

    fn handle(&mut self, msg: Broadcast, _ctx: &mut Context<Self>) {
        for user in self.users.values() {
            user.session_addr
                .do_send(Print {
                    message: msg.message.clone(),
                })
                .unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Broadcast, Connect, Disconnect, Print, Voting};
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    struct TestSession {
        pub messages: Arc<Mutex<Vec<String>>>,
    }

    impl Actor for TestSession {
        type Context = Context<Self>;
    }

    impl Handler<Print> for TestSession {
        type Result = ();

        fn handle(&mut self, msg: Print, _ctx: &mut Context<Self>) {
            let mut messages = self.messages.lock().unwrap();
            messages.push(msg.message);
        }
    }

    // Helper to create a session
    fn setup_session(messages: Arc<Mutex<Vec<String>>>) -> Addr<TestSession> {
        TestSession { messages }.start()
    }

    // Helper to connect a user
    async fn connect_user(
        game: &Addr<Game>,
        id: Uuid,
        nickname: &str,
        recipient: Recipient<Print>,
    ) {
        let connect_msg = Connect {
            id,
            nickname: nickname.to_string(),
            addr: recipient,
        };
        game.send(connect_msg).await.unwrap();
    }

    // Helper to lock and extract messages from a shared state
    fn lock_messages(messages: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
        messages.lock().unwrap().clone()
    }

    // Helper to validate messages
    fn validate_messages(messages: Vec<String>, expected: &[&str]) {
        assert_eq!(messages.len(), expected.len());
        for (i, &message) in expected.iter().enumerate() {
            assert_eq!(messages[i], message);
        }
    }

    #[actix_rt::test]
    async fn test_connect_handler() {
        let game = Game::new().start();
        let messages = Arc::new(Mutex::new(Vec::new()));

        let session = setup_session(messages.clone());

        connect_user(&game, Uuid::new_v4(), "test_user", session.recipient()).await;

        let msgs = lock_messages(&messages);
        validate_messages(msgs, &["Users: test_user"]);
    }

    #[actix_rt::test]
    async fn test_disconnect_handler() {
        let game = Game::new().start();
        let messages = Arc::new(Mutex::new(Vec::new()));

        let session = setup_session(messages.clone());
        let session2 = setup_session(messages.clone());

        let user1_id = Uuid::new_v4();
        let user2_id = Uuid::new_v4();

        connect_user(&game, user1_id, "test_user1", session.recipient()).await;
        connect_user(&game, user2_id, "test_user2", session2.recipient()).await;

        let disconnect_msg = Disconnect { id: user1_id };
        game.send(disconnect_msg).await.unwrap();

        let msgs = lock_messages(&messages);
        validate_messages(
            msgs,
            &[
                "Users: test_user1",
                "Users: test_user1, test_user2",
                "Users: test_user1, test_user2",
                "Users: test_user2",
            ],
        );
    }

    #[actix_rt::test]
    async fn test_voting_handler() {
        let game = Game::new().start();

        let messages1 = Arc::new(Mutex::new(Vec::new()));
        let session1 = setup_session(messages1.clone());

        let messages2 = Arc::new(Mutex::new(Vec::new()));
        let session2 = setup_session(messages2.clone());

        let user1_id = Uuid::new_v4();
        let user2_id = Uuid::new_v4();

        connect_user(&game, user1_id, "test_user1", session1.recipient()).await;
        connect_user(&game, user2_id, "test_user2", session2.recipient()).await;

        let voting_msg1 = Voting {
            id: user1_id,
            vote_text: "1".to_string(),
        };
        game.send(voting_msg1).await.unwrap();

        let voting_msg2 = Voting {
            id: user2_id,
            vote_text: "3".to_string(),
        };
        game.send(voting_msg2).await.unwrap();

        let msgs1 = lock_messages(&messages1);
        validate_messages(
            msgs1,
            &[
                "Users: test_user1",
                "Users: test_user1, test_user2",
                "You voted: 1",
                "Votes: test_user1: voted, test_user2: not voted",
                "Votes: test_user1: 1, test_user2: 3",
            ],
        );

        let msgs2 = lock_messages(&messages2);
        validate_messages(
            msgs2,
            &[
                "Users: test_user1, test_user2",
                "Votes: test_user1: voted, test_user2: not voted",
                "You voted: 3",
                "Votes: test_user1: 1, test_user2: 3",
            ],
        );
    }

    #[actix_rt::test]
    async fn test_broadcast_handler() {
        let game = Game::new().start();

        let messages1 = Arc::new(Mutex::new(Vec::new()));
        let session1 = setup_session(messages1.clone());

        let messages2 = Arc::new(Mutex::new(Vec::new()));
        let session2 = setup_session(messages2.clone());

        connect_user(&game, Uuid::new_v4(), "test_user1", session1.recipient()).await;
        connect_user(&game, Uuid::new_v4(), "test_user2", session2.recipient()).await;

        let broadcast_msg = Broadcast {
            message: "test_broadcast".to_string(),
        };
        game.send(broadcast_msg).await.unwrap();

        let msgs1 = lock_messages(&messages1);
        validate_messages(
            msgs1,
            &[
                "Users: test_user1",
                "Users: test_user1, test_user2",
                "test_broadcast",
            ],
        );

        let msgs2 = lock_messages(&messages2);
        validate_messages(msgs2, &["Users: test_user1, test_user2", "test_broadcast"]);
    }
}
