use super::game::{Command, ConnId, Msg, Vote};
use crate::session::Nickname;
use tokio::sync::{
    mpsc,
    oneshot::{self, error::RecvError},
};

#[cfg_attr(test, mockall::automock)]

pub trait CommandHandler: Clone {
    async fn connect(
        &self,
        conn_tx: mpsc::UnboundedSender<Msg>,
        nickname: Nickname,
    ) -> Result<ConnId, RecvError>;
    fn disconnect(&self, id: ConnId);
    async fn vote(&self, id: ConnId, vote: String);
}

#[cfg(test)]
impl Clone for MockCommandHandler {
    fn clone(&self) -> Self {
        MockCommandHandler::new()
    }
}

#[derive(Debug, Clone)]
pub struct GameHandle {
    pub cmd_tx: mpsc::UnboundedSender<Command>,
}

impl CommandHandler for GameHandle {
    async fn connect(
        &self,
        conn_tx: mpsc::UnboundedSender<Msg>,
        nickname: Nickname,
    ) -> Result<ConnId, RecvError> {
        let (res_tx, res_rx) = oneshot::channel();

        self.cmd_tx
            .send({
                let nickname = nickname.clone();
                Command::Connect {
                    conn_tx,
                    nickname,
                    res_tx,
                }
            })
            .unwrap_or_else(|err| {
                eprintln!(
                    "Failed to send Command::Connect for nickname {}: {}",
                    nickname, err
                );
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
