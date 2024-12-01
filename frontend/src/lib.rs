use gloo_net::websocket::{futures::WebSocket, Message};
use hand::render_cards;
pub use shared::{InboundMessage, OutboundMessage};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::wasm_bindgen;
use yew::{prelude::*, Renderer};

mod hand;
mod update;

pub enum Msg {
    Inbound(InboundMessage),
    Outbound(OutboundMessage),
}

pub struct Model {
    ws_sink: Option<Rc<RefCell<futures_util::stream::SplitSink<WebSocket, Message>>>>,
    user_lists: Vec<String>,
    messages: Vec<String>,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            ws_sink: None,
            user_lists: vec![],
            messages: vec![],
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        update::update(self, ctx, msg)
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div>
                <h1>{ "Planning Poker" }</h1>
                <div>
                    <button onclick={link.callback(|_| {
                        Msg::Inbound(InboundMessage::Connect { nickname: "Player1".into() } )
                    })}>
                        { "Conectar" }
                    </button>
                </div>
                <div>
                    <h2>{ "Users" }</h2>
                    {
                        for self.user_lists.iter().map(|user| html! { <p>{ user }</p> })
                    }
                    { render_cards(link) }
                </div>
            </div>
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    Renderer::<Model>::new().render();
}
