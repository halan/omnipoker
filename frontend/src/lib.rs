use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use shared::{InboundMessage, OutboundMessage};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::spawn_local;
use yew::{prelude::*, Renderer};

enum Msg {
    Inbound(InboundMessage),
    Outbound(OutboundMessage),
}

struct Model {
    ws_sink: Option<Rc<RefCell<futures_util::stream::SplitSink<WebSocket, Message>>>>,
    messages: Vec<String>,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            ws_sink: None,
            messages: vec![],
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Inbound(InboundMessage::Connect { nickname }) => {
                let link = ctx.link().clone();
                let ws = WebSocket::open("ws://127.0.0.1:8080/ws?mode=json").unwrap();

                let (write, mut read) = ws.split();
                self.ws_sink = Some(Rc::new(RefCell::new(write)));

                spawn_local(async move {
                    while let Some(Ok(message)) = read.next().await {
                        if let Message::Text(text) = message {
                            link.send_message(Msg::Outbound(
                                serde_json::from_str(&text).unwrap_or(OutboundMessage::Unknown),
                            ));
                        }
                    }
                });

                if let Some(ws_sink) = &self.ws_sink {
                    let ws_sink = ws_sink.clone();
                    spawn_local(async move {
                        let mut sink = ws_sink.borrow_mut();
                        if let Ok(text) =
                            serde_json::to_string(&InboundMessage::Connect { nickname })
                        {
                            let _ = sink.send(Message::Text(text)).await;
                        };
                    });
                }

                true
            }
            Msg::Inbound(inbound) => {
                if let Some(ws_sink) = &self.ws_sink {
                    let ws_sink = ws_sink.clone();
                    spawn_local(async move {
                        let mut sink = ws_sink.borrow_mut();
                        if let Ok(text) = serde_json::to_string(&inbound) {
                            sink.send(Message::Text(text)).await.unwrap();
                        };
                    });
                }
                true
            }
            Msg::Outbound(outbound) => {
                self.messages.push(outbound.to_string());
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let cards = Rc::new(vec![
            ("-", "joker", "?"),
            ("2", "spades", "2"),
            ("3", "diams", "3"),
            ("5", "clubs", "5"),
            ("8", "hearts", "8"),
            ("K", "clubs", "13"),
        ]);

        html! {
            <div>
                <h1>{ "Planning Poker" }</h1>
                <div>
                    <button onclick={link.callback(|_| Msg::Inbound(InboundMessage::Connect { nickname: "Player1".into() } ))}>{ "Conectar" }</button>
                </div>
                <div>
                    <h2>{ "Mensagens" }</h2>
                    { for self.messages.iter().rev().take(5).map(|message| html! { <p>{ message }</p> }) }
                    <div class="playingCards fourColours rotateHand">
                        <ul class="hand">
                        { for cards.iter().map(|(rank, suit, vote)| {
                            let suit_symbol = match *suit {
                                "spades" => "♠",
                                "hearts" => "♥",
                                "diams" => "♦",
                                "clubs" => "♣",
                                "joker" => "",
                                _ => "",
                            };
                            html! {
                                <li>
                                    <a
                                        onclick={link.callback({
                                            let vote = vote.to_string();
                                            move |_| Msg::Inbound(InboundMessage::Vote { value: vote.clone() })
                                        })}
                                        class={format!("card rank-{} {}", rank.to_lowercase(), suit.to_lowercase())}
                                        href="#"
                                    >
                                        <span class="rank">{ rank }</span>
                                        <span class="suit">{ suit_symbol }</span>
                                    </a>
                                </li>
                            }
                        })}
                        </ul>
                    </div>
                </div>
            </div>
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    Renderer::<Model>::new().render();
}
