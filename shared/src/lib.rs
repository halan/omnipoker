use bytestring::ByteString;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum OutboundMessage {
    UserList(Vec<String>),
    VotesList(Vec<(String, String)>),
    YourVote(String),
    Unknown,
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
            _ => "Unknown message".to_string(),
        };

        write!(f, "{}", text)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InboundMessage {
    Connect { nickname: String },
    Vote { value: String },
    Unknown,
}

impl InboundMessage {
    pub fn from_string(text: &str) -> Self {
        if text.starts_with("/join") {
            InboundMessage::Connect {
                nickname: text.split_whitespace().skip(1).collect(),
            }
        } else {
            InboundMessage::Vote { value: text.into() }
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