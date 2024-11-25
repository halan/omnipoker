use actix::prelude::*;
use uuid::Uuid;

use crate::session::Nickname;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Voting {
    pub id: Uuid,
    pub vote_text: String,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Connect {
    pub id: Uuid,
    pub addr: Recipient<Print>,
    pub nickname: Nickname,
}

#[derive(Message, Clone)]
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
