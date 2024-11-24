use actix::{prelude::*, Addr};
use uuid::Uuid;

use crate::session::{Nickname, Session};

#[derive(Message)]
#[rtype(result = "()")]
pub struct Voting {
    pub id: Uuid,
    pub vote_text: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub id: Uuid,
    pub addr: Addr<Session>,
    pub nickname: Nickname,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: Uuid,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Broadcast {
    pub message: String,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Print {
    pub message: String,
}
