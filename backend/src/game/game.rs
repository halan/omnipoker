use super::game_handle::*;
use crate::error::{Error, Result};
use shared::VoteStatus;
pub use shared::{OutboundMessage, UserStatus, Vote};
use std::{cmp::Ordering, collections::HashMap};
use tokio::sync::mpsc;

use uuid::Uuid;

pub type Nickname = String;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ConnId(Uuid);

impl ConnId {
    pub fn new() -> Self {
        ConnId(Uuid::new_v4())
    }
}

impl std::fmt::Display for ConnId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct User {
    nickname: Nickname,
    tx: mpsc::UnboundedSender<OutboundMessage>,
    vote: Vote,
    status: UserStatus,
    ord: usize,
}

impl User {
    pub fn vote(&mut self, vote: Vote) {
        self.vote = vote;
    }
}

type UsersMap = HashMap<ConnId, User>;

fn validate_nickname<'a>(nickname: &'a str, users: &UsersMap) -> Result<&'a str> {
    let nickname = nickname.trim();

    if nickname.is_empty() {
        log::error!("Nickname cannot be empty: {}", nickname);
        return Err(Error::NicknameCannotBeEmpty);
    }

    if users.values().any(|user| user.nickname == nickname) {
        log::error!("Nickname already in use: {}", nickname);
        return Err(Error::NicknameAlreadyInUse(nickname.into()));
    }

    let nickname = if nickname.len() > 20 {
        log::warn!("Nickname too long, truncating: {}", nickname);
        &nickname[..20]
    } else {
        nickname
    };

    Ok(nickname)
}

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

    pub async fn connect(
        &mut self,
        tx: mpsc::UnboundedSender<OutboundMessage>,
        nickname: &str,
    ) -> Result<ConnId> {
        log::info!("User identified: {}", nickname);

        let nickname = validate_nickname(nickname, &self.users)?;

        // register session with random connection ID
        let user = User {
            nickname: nickname.to_string(),
            tx,
            vote: Vote::Null,
            status: UserStatus::Active,
            ord: 0,
        };

        let conn_id = ConnId::new();

        self.users.insert(conn_id.clone(), user);
        self.broadcast(&self.users_summary())?;
        if self.anyone_voted() {
            self.broadcast(&self.votes_summary())?;
        }

        Ok(conn_id)
    }

    pub fn disconnect(&mut self, id: &ConnId) -> Result<()> {
        log::info!(
            "User disconnected: {}",
            self.users.get(&id).map_or("<None>", |user| &user.nickname)
        );
        self.users.remove(&id);
        self.broadcast(&self.users_summary())?;

        Ok(())
    }

    pub fn vote(&mut self, id: &ConnId, vote: &Vote) -> Result<()> {
        let max_ord = self.users.values().map(|user| user.ord).max().unwrap_or(0);
        self.users.get_mut(&id).map(|user| {
            user.vote(vote.clone());
            user.ord = max_ord + 1;
        });
        self.send_message(id, OutboundMessage::YourVote(vote.clone()))?;
        self.broadcast(&self.votes_summary())?;

        if self.all_voted() {
            self.reset_votes();
        }

        Ok(())
    }

    fn all_voted(&self) -> bool {
        self.users
            .values()
            .all(|user| user.vote.is_valid_vote() || matches!(user.status, UserStatus::Away))
    }

    fn anyone_voted(&self) -> bool {
        self.users
            .values()
            .any(|user| user.vote.is_valid_vote() && matches!(user.status, UserStatus::Active))
    }

    pub fn set_status(&mut self, id: &ConnId, status: &UserStatus) -> Result<()> {
        self.users
            .get_mut(&id)
            .map(|user| user.status = status.clone());

        self.send_message(id, OutboundMessage::YourStatus(status.clone()))?;
        self.broadcast(&self.users_summary())?;

        Ok(())
    }

    pub fn users_summary(&self) -> OutboundMessage {
        let mut users = self
            .users
            .values()
            .filter(|user| matches!(user.status, UserStatus::Active))
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
            .filter(|user| matches!(user.status, UserStatus::Active))
            .map(|user| (user.nickname.clone(), user.vote.status(), user.ord))
            .collect::<Vec<(String, VoteStatus, usize)>>();
        statuses.sort_by(|(nick_a, status_a, ord_a), (nick_b, status_b, ord_b)| {
            match (status_a, status_b) {
                (VoteStatus::NotVoted, VoteStatus::NotVoted) => nick_a.cmp(nick_b),
                (VoteStatus::NotVoted, _) => Ordering::Greater,
                (_, VoteStatus::NotVoted) => Ordering::Less,
                _ => ord_a.cmp(ord_b),
            }
        });

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
            .filter(|user| matches!(user.status, UserStatus::Active))
            .map(|user| (user.nickname.clone(), user.vote.clone(), user.ord))
            .collect::<Vec<(String, Vote, usize)>>();
        votes.sort_by(|(_, _, ord_a), (_, _, ord_b)| ord_a.cmp(ord_b));

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
        message: &OutboundMessage,
    ) -> Result<()> {
        for target in targets {
            target.send(message.clone())?;
        }

        Ok(())
    }

    pub fn broadcast(&self, message: &OutboundMessage) -> Result<()> {
        let targets: Vec<_> = self.users.values().map(|user| &user.tx).collect();
        self.send_to(targets, &message)?;

        Ok(())
    }

    pub fn send_message(&self, id: &ConnId, message: OutboundMessage) -> Result<()> {
        let user = self.users.get(&id).ok_or(Error::UserNotFound(id.clone()))?;
        self.send_to(vec![&user.tx], &message)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::{mpsc, oneshot};

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

            server.run().await;
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

    async fn connect_user(nickname: &str, handle: &GameHandle) -> Result<ConnId> {
        let (tx, _rx) = mpsc::unbounded_channel();
        let (res_tx, res_rx) = oneshot::channel();

        handle
            .cmd_tx
            .send(Command::Connect {
                conn_tx: tx,
                nickname: nickname.into(),
                res_tx: Some(res_tx),
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

        let conn_id = connect_user("Player1", &handle).await.unwrap();

        handle
            .cmd_tx
            .send(Command::Disconnect {
                conn_id: conn_id.clone(),
                res_tx: None,
            })
            .unwrap();

        // unlock the server
        shutdown_test_server(&handle, server_task).await;
        let server = server.lock().await;

        assert!(!server.users.contains_key(&conn_id));
        assert_eq!(server.users_summary(), OutboundMessage::UserList(vec![]));
    }

    #[tokio::test]
    async fn test_voting() {
        let (server, handle, server_task) = setup_test_server();

        let conn_id = connect_user("Player1", &handle).await.unwrap();
        let _ = connect_user("Player2", &handle).await;

        let vote = Vote::Option(2);

        handle
            .cmd_tx
            .send(Command::Vote {
                conn_id: conn_id.clone(),
                vote: vote.clone(),
                res_tx: None,
            })
            .unwrap();

        // unlock the server
        shutdown_test_server(&handle, server_task).await;
        let server = server.lock().await;

        assert_eq!(server.users.get(&conn_id).unwrap().vote, Vote::Option(2));
        assert_eq!(
            server.votes_summary(),
            OutboundMessage::VotesStatus(vec![
                ("Player1".into(), VoteStatus::Voted),
                ("Player2".into(), VoteStatus::NotVoted),
            ])
        );
    }
}
