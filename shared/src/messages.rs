pub use super::vote::{Vote, VoteStatus};
use bytestring::ByteString;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum UserStatus {
    Active,
    Away,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum OutboundMessage {
    UserList(Vec<String>),
    VotesResult(Vec<(String, Vote)>),
    VotesStatus(Vec<(String, VoteStatus)>),
    YourVote(Vote),
    YourStatus(UserStatus),
    Unknown,
    Error(String),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InboundMessage {
    Connect { nickname: String },
    Vote { value: Vote },
    SetStatus(UserStatus),
    Unknown,
}

impl fmt::Display for OutboundMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            OutboundMessage::UserList(users) => match users.len() {
                0 => "Users: nobody is active".to_string(),
                _ => format!("Users: {}", users.join(", ")),
            },
            OutboundMessage::VotesResult(votes) => {
                format!(
                    "Votes: {}",
                    votes
                        .iter()
                        .map(|(nickname, vote)| format!("{}: {}", nickname, vote))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            OutboundMessage::VotesStatus(statuses) => {
                format!(
                    "Votes: {}",
                    statuses
                        .iter()
                        .map(|(nickname, status)| format!("{}: {}", nickname, status))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            OutboundMessage::YourStatus(status) => match status {
                UserStatus::Active => "You are active".to_string(),
                UserStatus::Away => "You are away".to_string(),
            },
            OutboundMessage::YourVote(vote) => {
                format!("You voted: {}", vote)
            }
            _ => "Unknown message".to_string(),
        };

        write!(f, "{}", text)
    }
}

impl InboundMessage {
    pub fn from_string(text: &str) -> Self {
        let mut parts = text.split_whitespace();
        let (command, values) = (parts.next(), parts.collect::<Vec<_>>().join(" "));

        match (command, values.as_str()) {
            (Some("/join"), nickname) => InboundMessage::Connect {
                nickname: nickname.to_string(),
            },
            (Some("/setaway"), _) => InboundMessage::SetStatus(UserStatus::Away),
            (Some("/setback"), _) => InboundMessage::SetStatus(UserStatus::Active),
            (Some(vote), _) => InboundMessage::Vote { value: vote.into() },
            _ => InboundMessage::Unknown,
        }
    }
}

impl From<ByteString> for InboundMessage {
    fn from(text: ByteString) -> Self {
        InboundMessage::from_string(&text)
    }
}

impl From<String> for InboundMessage {
    fn from(text: String) -> Self {
        InboundMessage::from_string(&text)
    }
}
