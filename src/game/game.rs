use itertools::Itertools;
use log::info;
use rand::{thread_rng, Rng as _};
use std::{collections::HashMap, io, vec::IntoIter};
use tokio::sync::{mpsc, oneshot};

use super::command_handle::*;
pub use super::vote::Vote;

pub type ConnId = usize;
pub type Msg = String;
pub type Nickname = String;

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

    async fn connect(&mut self, tx: mpsc::UnboundedSender<Msg>, nickname: &str) -> ConnId {
        info!("Someone joined");

        // register session with random connection ID
        let id = thread_rng().gen::<ConnId>();
        let user = User {
            nickname: nickname.to_owned(),
            tx,
        };
        self.users.insert(id, user);
        self.broadcast(self.users_summary().as_str());

        id
    }

    pub fn disconnect(&mut self, id: ConnId) {
        self.users.remove(&id);
        self.votes.remove(&id);
        self.broadcast(self.users_summary().as_str());
    }

    pub fn vote(&mut self, id: ConnId, vote: Vote) {
        self.votes.insert(id, vote.clone());
        if let Vote::Option(vote_value) = vote {
            self.send_message(id, &format!("You voted: {}", vote_value));
        }
        self.broadcast(self.votes_summary().as_str());

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
        F: Fn(&String, &Vote) -> String,
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

    fn send_to(&self, targets: Vec<&mpsc::UnboundedSender<Msg>>, message: &str) {
        for target in targets {
            let _ = target.send(message.into());
        }
    }

    pub fn broadcast(&self, message: &str) {
        let targets: Vec<_> = self.users.values().map(|user| &user.tx).collect();
        self.send_to(targets, message);
    }

    pub fn send_message(&self, id: ConnId, message: &str) {
        if let Some(user) = self.users.get(&id) {
            self.send_to(vec![&user.tx], message);
        }
    }

    pub async fn run(&mut self) -> io::Result<()> {
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

                #[cfg(test)]
                Command::Shutdown => {
                    println!("Shutting down server.");
                    break;
                }
            }
        }

        Ok(())
    }
}

pub enum Command {
    Connect {
        conn_tx: mpsc::UnboundedSender<Msg>,
        nickname: String,
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

    #[cfg(test)]
    Shutdown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use tokio::sync::oneshot;

    fn setup_test_server() -> (
        Arc<Mutex<GameServer>>,
        GameHandle,
        actix_rt::task::JoinHandle<()>,
    ) {
        let (server, handle) = GameServer::new();
        let server = Arc::new(Mutex::new(server));

        let server_clone = Arc::clone(&server);
        let server_task = tokio::spawn(async move {
            let mut server = server_clone.lock().await;
            server.run().await.unwrap();
        });

        (server, handle, server_task)
    }

    async fn shutdown_test_server(
        handle: &GameHandle,
        server_task: actix_rt::task::JoinHandle<()>,
    ) {
        handle.cmd_tx.send(Command::Shutdown).unwrap();
        server_task.await.expect("Server task did not complete");
    }

    async fn connect_user(nickname: &str, handle: &GameHandle) -> ConnId {
        let (tx, _rx) = mpsc::unbounded_channel();
        let (res_tx, res_rx) = oneshot::channel();

        handle
            .cmd_tx
            .send(Command::Connect {
                conn_tx: tx,
                nickname: nickname.into(),
                res_tx,
            })
            .unwrap();

        res_rx.await.unwrap()
    }

    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_user_connection() {
        let (server, handle, server_task) = setup_test_server();
        let conn_id = connect_user("Player1", &handle).await;

        // unlock the server
        shutdown_test_server(&handle, server_task).await;

        let server = server.lock().await;
        assert!(server.users.contains_key(&conn_id));
        assert_eq!(server.users_summary(), "Users: Player1");
    }

    #[tokio::test]
    async fn test_user_disconnection() {
        let (server, handle, server_task) = setup_test_server();

        let conn_id = connect_user("Player1", &handle).await;

        handle.cmd_tx.send(Command::Disconnect { conn_id }).unwrap();

        // unlock the server
        shutdown_test_server(&handle, server_task).await;
        let server = server.lock().await;

        assert!(!server.users.contains_key(&conn_id));
        assert_eq!(server.users_summary(), "Users: ");
    }

    #[tokio::test]
    async fn test_voting() {
        let (server, handle, server_task) = setup_test_server();

        let conn_id = connect_user("Player1", &handle).await;
        let _ = connect_user("Player2", &handle).await;

        let vote = Vote::Option(2);

        let (vote_res_tx, vote_res_rx) = oneshot::channel();
        handle
            .cmd_tx
            .send(Command::Vote {
                conn_id,
                vote: vote.clone(),
                res_tx: vote_res_tx,
            })
            .unwrap();

        vote_res_rx.await.unwrap();

        // unlock the server
        shutdown_test_server(&handle, server_task).await;
        let server = server.lock().await;

        assert_eq!(server.votes.get(&conn_id), Some(&vote));
        assert_eq!(
            server.votes_summary(),
            "Votes: Player1: voted, Player2: not voted"
        );
    }
}
