use crate::{
    state::{Stage, State, StateAction},
    ws::{connect_websocket, send_message, WebSocketSink},
};
use gloo_net::websocket::WebSocketError;
use shared::{InboundMessage, OutboundMessage, Vote};
use std::borrow::Borrow;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub struct UsePlanningPokerReturn {
    pub state: State,
    pub ws_sink: UseStateHandle<Option<WebSocketSink>>,
    pub on_nickname_change: Callback<InputEvent>,
    pub connect_callback: Callback<SubmitEvent>,
    pub on_set_away: Callback<MouseEvent>,
    pub on_away_back: Callback<String>,
    pub on_vote: Callback<String>,
    pub on_remove_vote: Callback<String>,
}

#[hook]
pub fn use_planning_poker() -> UsePlanningPokerReturn {
    use shared::UserStatus;

    let ws_sink = use_state(|| None);
    let state = use_reducer(State::default);

    let on_nickname_change = {
        let state = state.clone();

        Callback::from(move |event: InputEvent| {
            if let Some(input) = event.target_dyn_into::<web_sys::HtmlInputElement>() {
                state.dispatch(StateAction::Connect(Some(input.value())));
            }
        })
    };

    let connect_callback = {
        let ws_sink = ws_sink.clone();
        let state = state.clone();

        Callback::from(move |event: SubmitEvent| {
            let state = state.clone();
            event.prevent_default();
            if state.nickname.is_none() {
                state.dispatch(StateAction::ConnectError(
                    "Nickname is required".to_string(),
                ));
                return;
            }

            if ws_sink.borrow().is_none() {
                if let Some(sink) = connect_websocket(
                    {
                        let state = state.clone();

                        move |outbound| match outbound {
                            OutboundMessage::UserList(list) => {
                                state.dispatch(StateAction::UpdateUserList(list));
                            }

                            OutboundMessage::VotesResult(results) => {
                                state.dispatch(StateAction::Result(Stage::Result(results)));
                            }

                            OutboundMessage::VotesStatus(statuses) => {
                                state.dispatch(StateAction::Status(Stage::Status(statuses)));
                            }

                            OutboundMessage::YourVote(vote) => {
                                state.dispatch(StateAction::YourVote(vote));
                            }

                            OutboundMessage::YourStatus(status) => {
                                state.dispatch(StateAction::YourStatus(status));
                            }

                            _ => {}
                        }
                    },
                    {
                        let state = state.clone();
                        // error
                        let ws_sink = ws_sink.clone();
                        move |err| {
                            ws_sink.set(None);
                            match err {
                                WebSocketError::ConnectionClose(e) => {
                                    state.dispatch(StateAction::ConnectError(e.reason));
                                }
                                WebSocketError::ConnectionError => {
                                    state.dispatch(StateAction::ConnectError(
                                        "The server is unreachable".to_string(),
                                    ));
                                }
                                _ => {}
                            }
                        }
                    },
                ) {
                    ws_sink.set(Some(sink.clone()));
                    let state = state.clone();

                    log::info!("Connected to websocket");
                    spawn_local(async move {
                        log::info!("Sending nickname");
                        if let Some(nickname) = state.nickname.clone() {
                            log::info!("Sending nickname: {}", nickname);
                            let message = InboundMessage::Connect { nickname };
                            send_message(&sink, &message).await;
                        }
                    });
                }
            }
        })
    };

    let on_set_away = {
        let ws_sink = ws_sink.clone();

        Callback::from(move |_| {
            if let Some(sink) = &*ws_sink {
                let sink = sink.clone();
                spawn_local(async move {
                    let message = InboundMessage::SetStatus(UserStatus::Away);
                    send_message(&sink, &message).await;
                });
            }
        })
    };

    let on_away_back = {
        let ws_sink = ws_sink.clone();

        Callback::from(move |_| {
            log::debug!("on_away_back");
            if let Some(sink) = &*ws_sink {
                let sink = sink.clone();
                spawn_local(async move {
                    let message = InboundMessage::SetStatus(UserStatus::Active);
                    send_message(&sink, &message).await;
                });
            }
        })
    };

    let on_vote = {
        let ws_sink = ws_sink.clone();

        Callback::from(move |vote: String| {
            if let Some(sink) = &*ws_sink {
                let sink = sink.clone();
                spawn_local(async move {
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

        Callback::from(move |_| {
            if let Some(sink) = &*ws_sink {
                let sink = sink.clone();
                spawn_local(async move {
                    let message = InboundMessage::Vote { value: Vote::Null };
                    send_message(&sink, &message).await;
                });
            }
        })
    };

    UsePlanningPokerReturn {
        state: (*state).clone(),
        ws_sink,
        on_nickname_change,
        connect_callback,
        on_set_away,
        on_away_back,
        on_vote,
        on_remove_vote,
    }
}
