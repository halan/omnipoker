use super::game::{Command, ConnId, Msg, Vote};
use tokio::sync::{
    mpsc,
    oneshot::{self, error::RecvError},
};

#[cfg_attr(test, mockall::automock)]

pub trait CommandHandler: Clone {
    async fn connect(
        &self,
        conn_tx: mpsc::UnboundedSender<Msg>,
        nickname: &str,
    ) -> Result<ConnId, RecvError>;
    fn disconnect(&self, id: ConnId);
    async fn vote(&self, id: ConnId, vote: &str);
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
        nickname: &str,
    ) -> Result<ConnId, RecvError> {
        let (res_tx, res_rx) = oneshot::channel();

        self.cmd_tx
            .send(Command::Connect {
                conn_tx,
                nickname: nickname.into(),
                res_tx,
            })
            .expect("Failed to send Command::Connect");

        res_rx.await
    }

    fn disconnect(&self, conn_id: ConnId) {
        self.cmd_tx
            .send(Command::Disconnect { conn_id })
            .expect("Failed to send Command::Disconnect");
    }

    async fn vote(&self, conn_id: ConnId, vote: &str) {
        let (res_tx, res_rx) = oneshot::channel();

        self.cmd_tx
            .send(Command::Vote {
                conn_id,
                vote: Vote::from(vote),
                res_tx,
            })
            .expect("Failed to send Command::Vote");

        res_rx.await.expect("Failed to receive ConnId");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Command;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_connect() {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        let game_handle = GameHandle { cmd_tx };

        let nickname = "Player1".to_string();
        let conn_tx = mpsc::unbounded_channel().0;

        let expected_connd_id = ConnId::new();

        tokio::spawn({
            let nickname = nickname.clone();
            async move {
                if let Some(Command::Connect {
                    conn_tx: _,
                    nickname: n,
                    res_tx: r,
                }) = cmd_rx.recv().await
                {
                    assert_eq!(n, nickname);
                    r.send(expected_connd_id).unwrap();
                }
            }
        });

        let conn_id = game_handle
            .connect(conn_tx, nickname.as_str())
            .await
            .expect("Failed to receive ConnId");

        assert_eq!(conn_id, expected_connd_id);
    }

    #[tokio::test]
    async fn test_vote() {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        let game_handle = GameHandle { cmd_tx };

        let conn_id = ConnId::new();
        let vote_value = "2";

        tokio::spawn(async move {
            if let Some(Command::Vote {
                conn_id: id,
                vote,
                res_tx,
            }) = cmd_rx.recv().await
            {
                assert_eq!(id, conn_id);
                assert_eq!(vote, Vote::Option(2));
                res_tx.send(ConnId::new()).unwrap();
            }
        });

        game_handle.vote(conn_id, vote_value).await;
    }
}
