use super::game::{Command, ConnId, OutboundMessage, Vote};
use shared::UserStatus;
use tokio::sync::{
    mpsc,
    oneshot::{self, error::RecvError},
};

#[derive(Debug, Clone)]
pub struct GameHandle {
    pub cmd_tx: mpsc::UnboundedSender<Command>,
}

impl GameHandle {
    pub async fn connect(
        &self,
        conn_tx: mpsc::UnboundedSender<OutboundMessage>,
        nickname: &str,
    ) -> Result<Result<ConnId, String>, RecvError> {
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

    pub fn disconnect(&self, conn_id: &ConnId) {
        self.cmd_tx
            .send(Command::Disconnect {
                conn_id: conn_id.clone(),
            })
            .expect("Failed to send Command::Disconnect");
    }

    pub async fn vote(&self, conn_id: &ConnId, vote: &Vote) {
        self.cmd_tx
            .send(Command::Vote {
                conn_id: conn_id.clone(),
                vote: vote.clone(),
            })
            .expect("Failed to send Command::Vote");
    }

    pub async fn set_status(&self, conn_id: &ConnId, status: &UserStatus) {
        self.cmd_tx
            .send(Command::SetAway {
                conn_id: conn_id.clone(),
                status: status.clone(),
            })
            .expect("Failed to send Command::SetAway");
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

        let expected_conn_id = ConnId::new();

        tokio::spawn({
            let nickname = nickname.clone();
            let expected_conn_id = expected_conn_id.clone();
            async move {
                if let Some(Command::Connect {
                    conn_tx: _,
                    nickname: n,
                    res_tx: r,
                }) = cmd_rx.recv().await
                {
                    assert_eq!(n, nickname);
                    r.send(Ok(expected_conn_id)).unwrap();
                }
            }
        });

        let conn_id = game_handle
            .connect(conn_tx, nickname.as_str())
            .await
            .expect("Failed to receive ConnId");

        assert_eq!(conn_id, Ok(expected_conn_id));
    }

    #[tokio::test]
    async fn test_vote() {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        let game_handle = GameHandle { cmd_tx };

        let conn_id = ConnId::new();
        let vote_value = Vote::new(2);

        tokio::spawn({
            let conn_id = conn_id.clone();
            async move {
                if let Some(Command::Vote { conn_id: id, vote }) = cmd_rx.recv().await {
                    assert_eq!(id, conn_id);
                    assert_eq!(vote, Vote::Option(2));
                }
            }
        });

        game_handle.vote(&conn_id, &vote_value).await;
    }
}
