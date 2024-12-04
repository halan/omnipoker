use crate::ws::{connect_websocket, send_message, WebSocketSink};
pub use shared::{InboundMessage, OutboundMessage, Vote, VoteStatus};
use std::{borrow::Borrow, rc::Rc};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub enum Stage {
    Init,
    Result(Vec<(String, Vote)>),
    Status(Vec<(String, VoteStatus)>),
}

impl Stage {
    pub fn count_votes(&self) -> usize {
        match self {
            Stage::Status(statuses) => statuses
                .iter()
                .filter(|(_, status)| matches!(status, VoteStatus::Voted))
                .count(),
            _ => 0,
        }
    }
}

pub struct UsePlanningPokerReturn {
    pub user_list: UseStateHandle<Vec<String>>,
    pub your_vote: UseStateHandle<Vote>,
    pub stage: Stage,
    pub is_rollback: bool,
    pub ws_sink: UseStateHandle<Option<WebSocketSink>>,
    pub nickname: UseStateHandle<Option<String>>,
    pub on_nickname_change: Callback<InputEvent>,
    pub connect_callback: Callback<SubmitEvent>,
    pub on_vote: Callback<String>,
    pub on_remove_vote: Callback<String>,
}

pub enum StateAction {
    Result(Stage),
    Status(Stage),
}
pub struct State {
    current_stage: Stage,
    previous_stage: Stage,
    pub is_rollback: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            current_stage: Stage::Init,
            previous_stage: Stage::Init,
            is_rollback: false,
        }
    }
}

impl Reducible for State {
    type Action = StateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            StateAction::Status(new_stage) => {
                let old_stage = self.current_stage.clone();
                Self {
                    current_stage: new_stage.clone(),
                    previous_stage: old_stage.clone(),
                    is_rollback: match (&new_stage, &old_stage) {
                        (new_stage @ Stage::Status(_), old_stage @ Stage::Status(_)) => {
                            new_stage.count_votes() < old_stage.count_votes()
                        }
                        _ => false,
                    },
                }
                .into()
            }
            StateAction::Result(new_stage) => Self {
                current_stage: new_stage.clone(),
                previous_stage: self.current_stage.clone(),
                is_rollback: false,
            }
            .into(),
        }
    }
}

#[hook]
pub fn use_planning_poker(server_addr: &'static str) -> UsePlanningPokerReturn {
    // TODO - move all state to the State struct
    let user_list = use_state(Vec::new);
    let your_vote = use_state(|| Vote::Null);
    let ws_sink = use_state(|| None);
    let nickname = use_state(|| None);

    let state = use_reducer(State::default);

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
        let state = state.clone();
        let nickname = nickname.clone();

        Callback::from(move |_| {
            let user_list = user_list.clone();
            let your_vote = your_vote.clone();
            let state = state.clone();
            let nickname = nickname.clone();

            if ws_sink.borrow().is_none() {
                if let Some(sink) = connect_websocket(
                    server_addr,
                    {
                        move |outbound| match outbound {
                            OutboundMessage::UserList(list) => {
                                user_list.set(list);
                            }

                            OutboundMessage::VotesResult(results) => {
                                state.dispatch(StateAction::Result(Stage::Result(results)));
                            }

                            OutboundMessage::VotesStatus(statuses) => {
                                state.dispatch(StateAction::Status(Stage::Status(statuses)));
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
        user_list,
        your_vote,
        stage: state.current_stage.clone(),
        is_rollback: state.is_rollback,
        ws_sink,
        nickname,
        on_nickname_change,
        connect_callback,
        on_vote,
        on_remove_vote,
    }
}
