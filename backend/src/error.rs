use crate::game::Command;
use crate::game::ConnId;
use shared::OutboundMessage;
use tokio::sync::{mpsc::error::SendError, oneshot::error::RecvError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    NicknameCannotBeEmpty,
    NicknameAlreadyInUse(String),
    UserNotFound(ConnId),
    SendMessage(SendError<OutboundMessage>),
    SendCommand(SendError<Command>),
    Recv(RecvError),
}

impl From<SendError<Command>> for Error {
    fn from(err: SendError<Command>) -> Self {
        Error::SendCommand(err)
    }
}

impl From<SendError<OutboundMessage>> for Error {
    fn from(err: SendError<OutboundMessage>) -> Self {
        Error::SendMessage(err)
    }
}

impl From<RecvError> for Error {
    fn from(err: RecvError) -> Self {
        Error::Recv(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NicknameCannotBeEmpty => write!(f, "Nickname cannot be empty"),
            Error::NicknameAlreadyInUse(nickname) => {
                write!(f, "Nickname {} is already in use", nickname)
            }
            Error::UserNotFound(conn_id) => write!(f, "User with id {} not found", conn_id),
            Error::SendMessage(err) => write!(f, "Failed to send message: {}", err),
            Error::SendCommand(err) => write!(f, "Failed to send command: {}", err),
            Error::Recv(err) => write!(f, "Failed to receive message: {}", err),
        }
    }
}
