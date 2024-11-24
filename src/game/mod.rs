use crate::session::{Nickname, Session};
use actix::{prelude::*, Addr};
use log::info;
use std::collections::HashMap;
use uuid::Uuid;

use self::vote::Vote;

mod handlers;
mod vote;

#[derive(Clone)]
struct User {
    nickname: Nickname,
    session_addr: Addr<Session>,
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

    pub fn all_voted(&self) -> bool {
        self.votes
            .iter()
            .filter(|(_, vote)| **vote != Vote::Null)
            .count()
            == self.users.len()
    }

    pub fn votes_summary(&self) -> String {
        if !self.all_voted() {
            return format!(
                "Votes: {}",
                self.users
                    .iter()
                    .map(|(id, _)| self.show_vote_status_from(id.clone()))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        format!(
            "Votes: {}",
            self.users
                .iter()
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
            self.users
                .values()
                .map(|user| user.nickname.clone())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub fn reset_votes(&mut self) {
        self.votes.clear();
    }

    pub fn remove_user(&mut self, id: Uuid) {
        self.users.remove(&id);
        self.votes.remove(&id);
    }
}

impl Actor for Game {
    type Context = Context<Self>;
}
