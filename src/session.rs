use actix::prelude::*;
use actix_web::{
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws::{self, Message, ProtocolError, WebsocketContext};
use log::info;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::{
    game::Game,
    messages::{Connect, Disconnect, Print, Voting},
};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub type Nickname = String;

pub struct Session {
    id: Uuid,
    nickname: Option<Nickname>,
    game_addr: Addr<Game>,
    hb: Instant,
}

impl Session {
    fn new(game_addr: Addr<Game>) -> Self {
        Session {
            id: Uuid::new_v4(),
            nickname: None,
            game_addr,
            hb: Instant::now(),
        }
    }

    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, move |actor, ctx| {
            if Instant::now().duration_since(actor.hb) > CLIENT_TIMEOUT {
                info!("Client heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }
            ctx.ping(b"PING");
        });
    }
}

impl Actor for Session {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Stream started");
        self.start_heartbeat(ctx);
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        self.game_addr.do_send(Disconnect {
            id: self.id.clone(),
        });
        info!("Stream stopped");
    }
}

impl StreamHandler<Result<Message, ProtocolError>> for Session {
    fn handle(&mut self, msg: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg)
            }
            Ok(Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(Message::Text(text)) => {
                if self.nickname == None {
                    if let ["/join", nickname] =
                        text.split_whitespace().collect::<Vec<_>>().as_slice()
                    {
                        self.nickname = nickname.to_string().into();
                        self.game_addr.do_send(Connect {
                            id: self.id.clone(),
                            addr: ctx.address(),
                            nickname: nickname.to_string(),
                        });
                    }

                    return;
                }

                self.game_addr.do_send(Voting {
                    id: self.id.clone(),
                    vote_text: text.to_string(),
                });
            }
            Ok(Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => (),
        }
    }
}

impl Handler<Print> for Session {
    type Result = ();

    fn handle(&mut self, msg: Print, ctx: &mut Self::Context) {
        ctx.text(msg.message);
    }
}

pub async fn handler(
    req: HttpRequest,
    payload: Payload,
    game: Data<Addr<Game>>,
) -> Result<HttpResponse, Error> {
    info!("Websocket connection initiated");
    ws::start(Session::new(game.get_ref().clone()), &req, payload)
}
