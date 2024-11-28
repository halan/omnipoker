use crate::session::Nickname;
use itertools::Itertools;
use log::info;
use rand::{thread_rng, Rng as _};
use std::{collections::HashMap, io, vec::IntoIter};
use tokio::sync::{mpsc, oneshot};

use super::command_handle::*;
pub use super::vote::Vote;

pub type ConnId = usize;
pub type Msg = String;

#[derive(Clone, Debug)]
pub struct User {
    nickname: Nickname,
    tx: mpsc::UnboundedSender<Msg>,
}

type UsersMap = HashMap<ConnId, User>;
type VotesMap = HashMap<ConnId, Vote>;

#[derive(Debug)]
pub struct GameServer {
    pub users: UsersMap,
    pub votes: VotesMap,
    pub cmd_rx: mpsc::UnboundedReceiver<Command>,
}

impl GameServer {
    pub fn new() -> (Self, GameHandle) {
        info!("Game started");

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        (
            Self {
                users: UsersMap::new(),
                votes: VotesMap::new(),
                cmd_rx,
            },
            GameHandle { cmd_tx },
        )
    }

    async fn connect(&mut self, tx: mpsc::UnboundedSender<Msg>, nickname: &Nickname) -> ConnId {
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

    fn all_voted(&self) -> bool {
        self.users.keys().all(|id| {
            self.votes
                .get(id)
                .map_or(false, |vote| vote.is_valid_vote())
        })
    }

    fn user_pairs_iter(&self) -> IntoIter<(ConnId, &User)> {
        self.users
            .iter()
            .map(|(id, user)| (id.clone(), user))
            .sorted_by(|a, b| a.1.nickname.cmp(&b.1.nickname))
    }

    pub fn users_summary(&self) -> String {
        self.format_summary("Users", |(_, user)| user.nickname.clone())
    }

    fn get_vote(&self, id: ConnId) -> Option<(&Nickname, &Vote)> {
        self.users
            .get(&id)
            .map(|user| (&user.nickname, self.votes.get(&id).unwrap_or(&Vote::Null)))
    }

    fn show_vote<F>(&self, id: ConnId, format_vote: F) -> String
    where
        F: Fn(&Nickname, &Vote) -> String,
    {
        self.get_vote(id)
            .map(|(nickname, vote)| format_vote(nickname, vote))
            .unwrap_or_else(String::new)
    }

    fn show_vote_from(&self, id: ConnId) -> String {
        self.show_vote(id, |nickname, vote| format!("{}: {}", nickname, vote))
    }

    fn show_vote_status_from(&self, id: ConnId) -> String {
        self.show_vote(id, |nickname, vote| {
            format!("{}: {}", nickname, vote.status())
        })
    }

    fn format_summary<F>(&self, label: &str, mapper: F) -> String
    where
        F: Fn((ConnId, &User)) -> String,
    {
        format!(
            "{}: {}",
            label,
            self.user_pairs_iter()
                .map(mapper)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub fn votes_summary(&self) -> String {
        let format_fn: Box<dyn Fn(ConnId) -> String> = if self.all_voted() {
            Box::new(|id: ConnId| self.show_vote_from(id))
        } else {
            Box::new(|id: ConnId| self.show_vote_status_from(id))
        };

        self.format_summary("Votes", |(id, _)| format_fn(id))
    }

    fn reset_votes(&mut self) {
        self.votes.clear();
    }

    fn send_to(&self, targets: Vec<&mpsc::UnboundedSender<Msg>>, message: String) {
        for target in targets {
            let _ = target.send(message.clone());
        }
    }

    pub fn broadcast(&self, message: String) {
        let targets: Vec<_> = self.users.values().map(|user| &user.tx).collect();
        self.send_to(targets, message);
    }

    pub fn send_message(&self, id: ConnId, message: String) {
        if let Some(user) = self.users.get(&id) {
            self.send_to(vec![&user.tx], message);
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
                    let conn_id = self.connect(conn_tx, &nickname).await;
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
