use super::command_handle::*;
use shared::VoteStatus;
pub use shared::{OutboundMessage, Vote};
use std::{collections::HashMap, io};
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
    ord: usize,
}

impl User {
    pub fn vote(&mut self, vote: Vote) {
        self.vote = vote;
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
        log::info!("Game started");

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
    ) -> Result<ConnId, String> {
        log::info!("User identified: {}", nickname);
        let nickname = nickname.trim();

        if nickname.is_empty() {
            log::error!("Nickname cannot be empty: {}", nickname);
            return Err("Nickname cannot be empty".into());
        }

        if self.users.values().any(|user| user.nickname == nickname) {
            log::error!("Nickname already in use: {}", nickname);
            return Err("Nickname already in use".into());
        }

        let nickname = if nickname.len() > 20usize {
            log::warn!("Nickname too long, truncating: {}", nickname);
            nickname[..20].to_string()
        } else {
            nickname.to_string()
        };

        // register session with random connection ID
        let id = ConnId::new();
        let user = User {
            nickname: nickname.clone(),
            tx,
            vote: Vote::Null,
            ord: 0,
        };

        self.users.insert(id, user);
        self.broadcast(self.users_summary());
        if self.anyone_voted() {
            self.broadcast(self.votes_summary());
        }

        Ok(id)
    }

    pub fn disconnect(&mut self, id: ConnId) {
        log::info!(
            "User disconnected: {}",
            self.users.get(&id).map_or("<None>", |user| &user.nickname)
        );
        self.users.remove(&id);
        self.broadcast(self.users_summary());
    }

    pub fn vote(&mut self, id: ConnId, vote: Vote) {
        let max_ord = self.users.values().map(|user| user.ord).max().unwrap_or(0);
        self.users.get_mut(&id).map(|user| {
            user.vote(vote.clone());
            user.ord = max_ord + 1;
        });
        self.send_message(id, OutboundMessage::YourVote(vote.into()));
        self.broadcast(self.votes_summary());

        if self.all_voted() {
            self.reset_votes();
        }
    }

    fn all_voted(&self) -> bool {
        self.users.values().all(|user| user.vote.is_valid_vote())
    }

    fn anyone_voted(&self) -> bool {
        self.users.values().any(|user| user.vote.is_valid_vote())
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
        if self.all_voted() {
            self.vote_result_summary()
        } else {
            self.vote_status_summary()
        }
    }

    fn vote_status_summary(&self) -> OutboundMessage {
        let mut statuses = self
            .users
            .values()
            .map(|user| (user.nickname.clone(), user.vote.status(), user.ord))
            .collect::<Vec<(String, VoteStatus, usize)>>();
        statuses.sort_by(
            |(_, status_a, ord_a), (_, status_b, ord_b)| match (status_a, status_b) {
                (VoteStatus::NotVoted, _) => std::cmp::Ordering::Greater,
                (_, VoteStatus::NotVoted) => std::cmp::Ordering::Less,
                _ => ord_a.cmp(ord_b),
            },
        );

        OutboundMessage::VotesStatus(
            statuses
                .iter()
                .map(|(a, b, _)| (a.clone(), b.clone()))
                .collect(),
        )
    }

    fn vote_result_summary(&self) -> OutboundMessage {
        let mut votes = self
            .users
            .values()
            .map(|user| (user.nickname.clone(), user.vote.clone(), user.ord))
            .collect::<Vec<(String, Vote, usize)>>();
        votes.sort_by(
            |(_, vote_a, ord_a), (_, vote_b, ord_b)| match (vote_a, vote_b) {
                (Vote::Null, _) => std::cmp::Ordering::Greater,
                (_, Vote::Null) => std::cmp::Ordering::Less,
                _ => ord_a.cmp(ord_b),
            },
        );

        OutboundMessage::VotesResult(
            votes
                .iter()
                .map(|(a, b, _)| (a.clone(), b.clone()))
                .collect(),
        )
    }

    fn reset_votes(&mut self) {
        self.users.iter_mut().for_each(|(_, user)| {
            user.vote = Vote::Null;
            user.ord = 0;
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
                    let result = self.connect(conn_tx, &nickname).await;
                    let _ = res_tx.send(result);
                }

                Command::Disconnect { conn_id } => {
                    self.disconnect(conn_id);
                }

                Command::Vote { conn_id, vote } => {
                    self.vote(conn_id, vote);
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
        res_tx: oneshot::Sender<Result<ConnId, String>>,
    },

    Disconnect {
        conn_id: ConnId,
    },

    Vote {
        conn_id: ConnId,
        vote: Vote,
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

    async fn connect_user(nickname: &str, handle: &GameHandle) -> Result<ConnId, String> {
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
        assert!(server.users.contains_key(&conn_id.unwrap()));
        assert_eq!(
            server.users_summary(),
            OutboundMessage::UserList(vec!["Player1".into()])
        );
    }

    #[tokio::test]
    async fn test_user_disconnection() {
        let (server, handle, server_task) = setup_test_server();

        let conn_id = connect_user("Player1", &handle).await;

        handle
            .cmd_tx
            .send(Command::Disconnect {
                conn_id: conn_id.clone().unwrap(),
            })
            .unwrap();

        // unlock the server
        shutdown_test_server(&handle, server_task).await;
        let server = server.lock().await;

        assert!(!server.users.contains_key(&conn_id.unwrap()));
        assert_eq!(server.users_summary(), OutboundMessage::UserList(vec![]));
    }

    #[tokio::test]
    async fn test_voting() {
        let (server, handle, server_task) = setup_test_server();

        let conn_id = connect_user("Player1", &handle).await;
        let _ = connect_user("Player2", &handle).await;

        let vote = Vote::Option(2);

        handle
            .cmd_tx
            .send(Command::Vote {
                conn_id: *conn_id.as_ref().unwrap(),
                vote: vote.clone(),
            })
            .unwrap();

        // unlock the server
        shutdown_test_server(&handle, server_task).await;
        let server = server.lock().await;

        assert_eq!(
            server.users.get(&conn_id.unwrap()).unwrap().vote,
            Vote::Option(2)
        );
        assert_eq!(
            server.votes_summary(),
            OutboundMessage::VotesStatus(vec![
                ("Player1".into(), VoteStatus::Voted),
                ("Player2".into(), VoteStatus::NotVoted),
            ])
        );
    }
}
