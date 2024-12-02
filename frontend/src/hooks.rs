use crate::ws::{connect_websocket, send_message, WebSocketSink};
pub use shared::{InboundMessage, OutboundMessage, Vote, VoteStatus};
use std::borrow::Borrow;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum Stage {
    Init,
    Result(Vec<(String, Vote)>),
    Status(Vec<(String, VoteStatus)>),
}

pub struct UsePlanningPokerReturn {
    pub user_list: UseStateHandle<Vec<String>>,
    pub your_vote: UseStateHandle<Vote>,
    pub stage: UseStateHandle<Stage>,
    pub ws_sink: UseStateHandle<Option<WebSocketSink>>,
    pub nickname: UseStateHandle<Option<String>>,
    pub on_nickname_change: Callback<InputEvent>,
    pub connect_callback: Callback<MouseEvent>,
    pub on_vote: Callback<String>,
    pub on_remove_vote: Callback<String>,
}

#[hook]
pub fn use_planning_poker(server_addr: &'static str) -> UsePlanningPokerReturn {
    let user_list = use_state(Vec::new);
    let your_vote = use_state(|| Vote::Null);
    let stage = use_state(|| Stage::Init);
    let ws_sink = use_state(|| None);
    let nickname = use_state(|| None);

    let on_nickname_change = {
        let nickname = nickname.clone();
        Callback::from(move |event: InputEvent| {
            if let Some(input) = event.target_dyn_into::<web_sys::HtmlInputElement>() {
                nickname.set(Some(input.value()));
            }
        })
    };

    let connect_callback = {
        let ws_sink = ws_sink.clone();
        let user_list = user_list.clone();
        let your_vote = your_vote.clone();
        let stage = stage.clone();
        let nickname = nickname.clone();

        Callback::from(move |_| {
            let user_list = user_list.clone();
            let your_vote = your_vote.clone();
            let stage = stage.clone();
            let nickname = nickname.clone();

            if ws_sink.borrow().is_none() {
                if let Some(sink) = connect_websocket(
                    server_addr,
                    {
                        // message
                        let user_list = user_list.clone();
                        move |outbound| match outbound {
                            OutboundMessage::UserList(list) => {
                                user_list.set(list);
                            }

                            OutboundMessage::VotesResult(results) => {
                                stage.set(Stage::Result(results));
                            }

                            OutboundMessage::VotesStatus(statuses) => {
                                stage.set(Stage::Status(statuses));
                            }

                            OutboundMessage::YourVote(vote) => {
                                your_vote.set(vote);
                            }
                            _ => {}
                        }
                    },
                    {
                        // error
                        let ws_sink = ws_sink.clone();
                        move |_| {
                            ws_sink.set(None);
                        }
                    },
                ) {
                    ws_sink.set(Some(sink.clone()));

                    wasm_bindgen_futures::spawn_local(async move {
                        if let Some(nickname) = &*nickname {
                            let message = InboundMessage::Connect {
                                nickname: nickname.to_string(),
                            };
                            send_message(&sink, &message).await;
                        }
                    });
                }
            }
        })
    };

    let on_vote = {
        let ws_sink = ws_sink.clone();

        Callback::from(move |vote: String| {
            if let Some(sink) = &*ws_sink {
                let sink = sink.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let message = InboundMessage::Vote {
                        value: Vote::from(vote),
                    };
                    send_message(&sink, &message).await;
                });
            }
        })
    };

    let on_remove_vote = {
        let ws_sink = ws_sink.clone();

        Callback::from(move |_: String| {
            if let Some(sink) = &*ws_sink {
                let sink = sink.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let message = InboundMessage::Vote { value: Vote::Null };
                    send_message(&sink, &message).await;
                });
            }
        })
    };

    UsePlanningPokerReturn {
        user_list,
        your_vote,
        stage,
        ws_sink,
        nickname,
        on_nickname_change,
        connect_callback,
        on_vote,
        on_remove_vote,
    }
}
