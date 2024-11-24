use actix::prelude::*;
use log::info;

use crate::{
    game::{vote::Vote, Game, User},
    messages::{Broadcast, Connect, Disconnect, Print, Voting},
};

impl Handler<Connect> for Game {
    type Result = ();

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) {
        self.users.insert(
            msg.id,
            User {
                nickname: msg.nickname.clone(),
                session_addr: msg.addr,
            },
        );
        self.handle(
            Broadcast {
                message: self.users_summary(),
            },
            ctx,
        );
        info!("User connected: {}", msg.nickname);
    }
}

impl Handler<Disconnect> for Game {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, ctx: &mut Context<Self>) {
        let nickname = self.users.get(&msg.id).unwrap().nickname.clone();
        self.remove_user(msg.id);
        self.handle(
            Broadcast {
                message: self.users_summary(),
            },
            ctx,
        );
        info!("User disconnected: {}", nickname);
    }
}

impl Handler<Voting> for Game {
    type Result = ();

    fn handle(&mut self, msg: Voting, ctx: &mut Context<Self>) {
        let vote = Vote::from(msg.vote_text.as_str());

        if let Some(user) = self.users.get(&msg.id) {
            self.votes.insert(msg.id.clone(), vote.clone());

            if let Vote::Option(vote_value) = vote {
                user.session_addr.do_send(Print {
                    message: format!("You voted: {}", vote_value),
                });
            }
        }

        self.handle(
            Broadcast {
                message: self.votes_summary(),
            },
            ctx,
        );

        if self.all_voted() {
            self.reset_votes();
        }
    }
}

impl Handler<Broadcast> for Game {
    type Result = ();

    fn handle(&mut self, msg: Broadcast, _ctx: &mut Context<Self>) {
        for user in self.users.values() {
            user.session_addr.do_send(Print {
                message: msg.message.clone(),
            });
        }
    }
}
