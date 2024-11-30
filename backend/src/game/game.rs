use super::command_handle::*;
pub use super::vote::Vote;
use log::info;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, io};
use tokio::sync::{mpsc, oneshot};

use uuid::Uuid;

pub type Nickname = String;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub struct ConnId(Uuid);

impl ConnId {
    pub fn new() -> Self {
        ConnId(Uuid::new_v4())
    }
}

#[derive(Clone, Debug)]
pub struct User {
    nickname: Nickname,
    tx: mpsc::UnboundedSender<OutboundMessage>,
    vote: Vote,
}

impl User {
    pub fn vote(&mut self, vote: Vote) {
        self.vote = vote;
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum OutboundMessage {
    UserList(Vec<String>),
    VotesList(Vec<(String, String)>),
    YourVote(String),
}

impl fmt::Display for OutboundMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            OutboundMessage::UserList(users) => {
                format!("Users: {}", users.join(", "))
            }
            OutboundMessage::VotesList(votes) => {
                format!(
                    "Votes: {}",
                    votes
                        .iter()
                        .map(|(nickname, vote)| format!("{}: {}", nickname, vote))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            OutboundMessage::YourVote(vote) => {
                format!("You voted: {}", vote)
            }
        };

        write!(f, "{}", text)
    }
}

type UsersMap = HashMap<ConnId, User>;

#[derive(Debug)]
pub struct GameServer {
    pub users: UsersMap,
    pub cmd_rx: mpsc::UnboundedReceiver<Command>,
}

impl GameServer {
    pub fn new() -> (Self, GameHandle) {
        info!("Game started");

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        (
            Self {
                users: UsersMap::new(),
                cmd_rx,
            },
            GameHandle { cmd_tx },
        )
    }

    async fn connect(
        &mut self,
        tx: mpsc::UnboundedSender<OutboundMessage>,
        nickname: &str,
    ) -> ConnId {
        info!("User identified: {}", nickname);

        // register session with random connection ID
        let id = ConnId::new();
        let user = User {
            nickname: nickname.to_owned(),
            tx,
            vote: Vote::Null,
        };
        self.users.insert(id, user);
        self.broadcast(self.users_summary());

        id
    }

    pub fn disconnect(&mut self, id: ConnId) {
        info!(
            "User disconnected: {}",
            self.users.get(&id).map_or("<None>", |user| &user.nickname)
        );
        self.users.remove(&id);
        self.broadcast(self.users_summary());
    }

    pub fn vote(&mut self, id: ConnId, vote: Vote) {
        self.users.get_mut(&id).map(|user| user.vote(vote.clone()));

        if let Vote::Option(vote) = vote {
            self.send_message(id, OutboundMessage::YourVote(vote.to_string()));
        }
        self.broadcast(self.votes_summary());

        if self.all_voted() {
            self.reset_votes();
        }
    }

    fn all_voted(&self) -> bool {
        self.users.values().all(|user| user.vote.is_valid_vote())
    }

    pub fn users_summary(&self) -> OutboundMessage {
        let mut users = self
            .users
            .values()
            .map(|user| user.nickname.clone())
            .collect::<Vec<String>>();

        users.sort();

        OutboundMessage::UserList(users)
    }

    pub fn votes_summary(&self) -> OutboundMessage {
        let mut votes = self
            .users
            .values()
            .map(|user| {
                let vote_value = if self.all_voted() {
                    user.vote.to_string()
                } else {
                    user.vote.status().to_string()
                };

                (user.nickname.clone(), vote_value)
            })
            .collect::<Vec<(String, String)>>();

        votes.sort_by(|(a, _), (b, _)| a.cmp(b));

        OutboundMessage::VotesList(votes)
    }

    fn reset_votes(&mut self) {
        self.users.iter_mut().for_each(|(_, user)| {
            user.vote = Vote::Null;
        });
    }

    fn send_to(
        &self,
        targets: Vec<&mpsc::UnboundedSender<OutboundMessage>>,
        message: OutboundMessage,
    ) {
        for target in targets {
            let _ = target.send(message.clone());
        }
    }

    pub fn broadcast(&self, message: OutboundMessage) {
        let targets: Vec<_> = self.users.values().map(|user| &user.tx).collect();
        self.send_to(targets, message);
    }

    pub fn send_message(&self, id: ConnId, message: OutboundMessage) {
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
        conn_tx: mpsc::UnboundedSender<OutboundMessage>,
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
        assert_eq!(
            server.users_summary(),
            OutboundMessage::UserList(vec!["Player1".into()])
        );
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
        assert_eq!(server.users_summary(), OutboundMessage::UserList(vec![]));
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
                conn_id: conn_id,
                vote: vote.clone(),
                res_tx: vote_res_tx,
            })
            .unwrap();

        vote_res_rx.await.unwrap();

        // unlock the server
        shutdown_test_server(&handle, server_task).await;
        let server = server.lock().await;

        assert_eq!(server.users.get(&conn_id).unwrap().vote, Vote::Option(2));
        assert_eq!(
            server.votes_summary(),
            OutboundMessage::VotesList(vec![
                ("Player1".into(), "voted".into()),
                ("Player2".into(), "not voted".into())
            ])
        );
    }
}
