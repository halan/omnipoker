use itertools::Itertools;
use log::info;
use mockall::*;
use rand::{thread_rng, Rng as _};
use std::{collections::HashMap, io, vec::IntoIter};
use tokio::sync::{
    mpsc,
    oneshot::{self, error::RecvError},
};

use self::vote::Vote;
use crate::session::Nickname;

pub mod vote;

#[derive(Clone, Debug)]
pub struct User {
    nickname: Nickname,
    tx: mpsc::UnboundedSender<Msg>,
}

#[derive(Debug)]
pub struct GameServer {
    pub users: HashMap<ConnId, User>,
    pub votes: HashMap<ConnId, Vote>,
    pub cmd_rx: mpsc::UnboundedReceiver<Command>,
}

impl GameServer {
    pub fn new() -> (Self, GameHandle) {
        info!("Game started");

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        (
            Self {
                users: HashMap::new(),
                votes: HashMap::new(),
                cmd_rx,
            },
            GameHandle { cmd_tx },
        )
    }

    async fn connect(&mut self, tx: mpsc::UnboundedSender<Msg>, nickname: Nickname) -> ConnId {
        info!("Someone joined");

        // register session with random connection ID
        let id = thread_rng().gen::<ConnId>();
        let user = User {
            nickname: nickname.to_owned(),
            tx,
        };
        self.users.insert(id, user);
        self.broadcast(self.users_summary());

        id
    }

    pub fn disconnect(&mut self, id: ConnId) {
        self.users.remove(&id);
        self.votes.remove(&id);
        self.broadcast(self.users_summary());
    }

    pub fn vote(&mut self, id: ConnId, vote: Vote) {
        self.votes.insert(id, vote.clone());
        if let Vote::Option(vote_value) = vote {
            self.send_message(id, format!("You voted: {}", vote_value));
        }
        self.broadcast(self.votes_summary());

        if self.all_voted() {
            self.reset_votes();
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

    fn user_pairs_iter(&self) -> IntoIter<(ConnId, &User)> {
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

    pub fn get_vote(&self, id: ConnId) -> (&Nickname, &Vote) {
        (
            &self.users.get(&id).unwrap().nickname,
            &self.votes.get(&id).unwrap_or(&Vote::Null),
        )
    }

    pub fn show_vote_from(&self, id: ConnId) -> String {
        let (nickname, vote) = self.get_vote(id);
        format!("{}: {}", nickname, vote)
    }

    pub fn show_vote_status_from(&self, id: ConnId) -> String {
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

    pub fn broadcast(&self, message: String) {
        for user in self.users.values() {
            let _ = user.tx.send(message.clone());
        }
    }

    pub fn send_message(&self, id: ConnId, message: String) {
        if let Some(user) = self.users.get(&id) {
            let _ = user.tx.send(message);
        }
    }

    pub async fn run(mut self) -> io::Result<()> {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                Command::Connect {
                    conn_tx,
                    nickname,
                    res_tx,
                } => {
                    let conn_id = self.connect(conn_tx, nickname).await;
                    let _ = res_tx.send(conn_id);
                }

                Command::Disconnect { conn_id } => {
                    self.disconnect(conn_id);
                }

                Command::Vote {
                    conn_id,
                    vote,
                    res_tx,
                } => {
                    self.vote(conn_id, vote);
                    let _ = res_tx.send(conn_id);
                }
            }
        }

        Ok(())
    }
}

pub enum Command {
    Connect {
        conn_tx: mpsc::UnboundedSender<Msg>,
        nickname: Nickname,
        res_tx: oneshot::Sender<ConnId>,
    },

    Disconnect {
        conn_id: ConnId,
    },

    Vote {
        conn_id: ConnId,
        vote: Vote,
        res_tx: oneshot::Sender<ConnId>,
    },
}
pub type ConnId = usize;
pub type Msg = String;

#[automock]
pub trait CommandHandler: Clone {
    async fn connect(
        &self,
        conn_tx: mpsc::UnboundedSender<Msg>,
        nickname: Nickname,
    ) -> Result<ConnId, RecvError>;
    fn disconnect(&self, id: ConnId);
    async fn vote(&self, id: ConnId, vote: String);
}

impl Clone for MockCommandHandler {
    fn clone(&self) -> Self {
        MockCommandHandler::new()
    }
}

#[derive(Debug, Clone)]
pub struct GameHandle {
    cmd_tx: mpsc::UnboundedSender<Command>,
}

impl CommandHandler for GameHandle {
    async fn connect(
        &self,
        conn_tx: mpsc::UnboundedSender<Msg>,
        nickname: Nickname,
    ) -> Result<ConnId, RecvError> {
        let (res_tx, res_rx) = oneshot::channel();

        self.cmd_tx
            .send(Command::Connect {
                conn_tx,
                nickname,
                res_tx,
            })
            .unwrap_or_else(|err| {
                eprintln!("Failed to send command: {}", err);
            });

        res_rx.await
    }

    fn disconnect(&self, conn_id: ConnId) {
        self.cmd_tx.send(Command::Disconnect { conn_id }).unwrap();
    }

    async fn vote(&self, conn_id: ConnId, vote: String) {
        let (res_tx, res_rx) = oneshot::channel();

        self.cmd_tx
            .send(Command::Vote {
                conn_id,
                vote: Vote::from(vote.as_str()),
                res_tx,
            })
            .unwrap();

        res_rx.await.unwrap();
    }
}
