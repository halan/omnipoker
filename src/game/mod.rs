use crate::{messages::Print, session::Nickname};
use actix::prelude::*;
use itertools::Itertools;
use log::info;
use std::{collections::HashMap, vec::IntoIter};
use uuid::Uuid;

use self::vote::Vote;

mod handlers;
mod vote;

#[derive(Clone, Debug)]
struct User {
    nickname: Nickname,
    session_addr: Recipient<Print>,
}

#[derive(Clone)]
pub struct Game {
    users: HashMap<Uuid, User>,
    votes: HashMap<Uuid, Vote>,
}

impl Game {
    pub fn new() -> Self {
        info!("Game created");
        Game {
            users: HashMap::new(),
            votes: HashMap::new(),
        }
    }

    pub fn count_valid_votes(&self) -> usize {
        self.votes
            .values()
            .filter(|vote| **vote != Vote::Null)
            .count()
    }

    pub fn all_voted(&self) -> bool {
        self.count_valid_votes() == self.users.len()
    }

    fn user_pairs_iter(&self) -> IntoIter<(Uuid, &User)> {
        self.users
            .iter()
            .map(|(id, user)| (id.clone(), user))
            .sorted_by(|a, b| a.1.nickname.cmp(&b.1.nickname))
    }

    pub fn votes_summary(&self) -> String {
        if !self.all_voted() {
            return format!(
                "Votes: {}",
                self.user_pairs_iter()
                    .map(|(id, _)| self.show_vote_status_from(id.clone()))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        format!(
            "Votes: {}",
            self.user_pairs_iter()
                .map(|(id, _)| self.show_vote_from(id.clone()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub fn get_vote(&self, id: Uuid) -> (&Nickname, &Vote) {
        (
            &self.users.get(&id).unwrap().nickname,
            &self.votes.get(&id).unwrap_or(&Vote::Null),
        )
    }

    pub fn show_vote_from(&self, id: Uuid) -> String {
        let (nickname, vote) = self.get_vote(id);
        format!("{}: {}", nickname, vote)
    }

    pub fn show_vote_status_from(&self, id: Uuid) -> String {
        let (nickname, vote) = self.get_vote(id);
        format!("{}: {}", nickname, vote.status())
    }

    pub fn users_summary(&self) -> String {
        format!(
            "Users: {}",
            self.user_pairs_iter()
                .map(|(_, user)| user.nickname.clone())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub fn reset_votes(&mut self) {
        self.votes.clear();
    }

    pub fn add_user(&mut self, id: Uuid, nickname: Nickname, addr: Recipient<Print>) {
        self.users.insert(
            id,
            User {
                nickname: nickname.clone(),
                session_addr: addr,
            },
        );
    }

    pub fn remove_user(&mut self, id: Uuid) {
        self.users.remove(&id);
        self.votes.remove(&id);
    }
}

impl Actor for Game {
    type Context = Context<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSession;

    impl Actor for TestSession {
        type Context = Context<Self>;
    }

    impl Handler<Print> for TestSession {
        type Result = ();

        fn handle(&mut self, _: Print, _: &mut Context<Self>) {
            ()
        }
    }

    fn setup_user(nickname: &str) -> User {
        User {
            nickname: nickname.into(),
            session_addr: TestSession {}.start().recipient::<Print>(),
        }
    }

    #[test]
    fn test_game_new() {
        let game = Game::new();
        assert_eq!(game.users.len(), 0);
        assert_eq!(game.votes.len(), 0);
    }

    #[test]
    fn test_game_count_valid_votes() {
        let mut game = Game::new();
        game.votes.insert(Uuid::new_v4(), Vote::Option(1));
        game.votes.insert(Uuid::new_v4(), Vote::Option(2));
        game.votes.insert(Uuid::new_v4(), Vote::Null);
        assert_eq!(game.count_valid_votes(), 2);
    }

    #[actix_rt::test]
    async fn test_game_all_voted() {
        let mut game = Game::new();
        game.users.insert(Uuid::new_v4(), setup_user("Alice"));
        game.users.insert(Uuid::new_v4(), setup_user("Bob"));
        game.votes.insert(Uuid::new_v4(), Vote::Option(1));
        game.votes.insert(Uuid::new_v4(), Vote::Option(2));
        assert_eq!(game.all_voted(), true);
    }

    #[actix_rt::test]
    async fn test_game_user_pairs_iter() {
        let mut game = Game::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        game.users.insert(id1, setup_user("Bob"));
        game.users.insert(id2, setup_user("Alice"));

        let pairs = game.user_pairs_iter().collect::<Vec<_>>();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, id2);
        assert_eq!(pairs[1].0, id1);
    }

    #[actix_rt::test]

    async fn test_game_votes_summary() {
        let mut game = Game::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        game.users.insert(id1, setup_user("Bob"));
        game.users.insert(id2, setup_user("Alice"));
        game.votes.insert(id1, Vote::Option(1));
        game.votes.insert(id2, Vote::Option(2));
        assert_eq!(game.votes_summary(), "Votes: Alice: 2, Bob: 1");
    }
}
