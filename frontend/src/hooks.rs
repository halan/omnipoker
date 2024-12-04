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

    pub fn your_vote_status(&self, nickname: &Option<String>) -> VoteStatus {
        match self {
            Stage::Status(statuses) => statuses
                .iter()
                .find(|(user, _)| Some(user) == nickname.as_ref())
                .map(|(_, status)| status.clone())
                .unwrap_or(VoteStatus::NotVoted),
            _ => VoteStatus::NotVoted,
        }
    }
}

pub struct UsePlanningPokerReturn {
    pub state: State,
    pub ws_sink: UseStateHandle<Option<WebSocketSink>>,
    pub on_nickname_change: Callback<InputEvent>,
    pub connect_callback: Callback<SubmitEvent>,
    pub on_vote: Callback<String>,
    pub on_remove_vote: Callback<String>,
}

pub enum StateAction {
    Result(Stage),
    Status(Stage),
    Connect(String),
    YourVote(Vote),
    UpdateUserList(Vec<String>),
}
#[derive(Clone)]
pub struct State {
    pub stage: Stage,
    pub is_rollback: bool,
    pub nickname: Option<String>,
    pub your_vote: Vote,
    pub user_list: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            stage: Stage::Init,
            nickname: None,
            your_vote: Vote::Null,
            is_rollback: false,
            user_list: Vec::new(),
        }
    }
}

impl Reducible for State {
    type Action = StateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            StateAction::Status(new_stage) => {
                let old_stage = self.stage.clone();
                Self {
                    stage: new_stage.clone(),
                    is_rollback: match (&new_stage, &old_stage) {
                        (new_stage @ Stage::Status(_), old_stage @ Stage::Status(_)) => {
                            new_stage.count_votes() < old_stage.count_votes()
                        }
                        _ => false,
                    },
                    your_vote: match new_stage.your_vote_status(&self.nickname) {
                        VoteStatus::Voted => self.your_vote.clone(),
                        _ => Vote::Null,
                    },
                    ..(*self).clone()
                }
            }
            StateAction::Result(new_stage) => Self {
                stage: new_stage.clone(),
                is_rollback: false,
                ..(*self).clone()
            },
            StateAction::Connect(nickname) => Self {
                nickname: Some(nickname),
                ..(*self).clone()
            },
            StateAction::YourVote(vote) => Self {
                your_vote: vote,
                ..(*self).clone()
            },
            StateAction::UpdateUserList(list) => Self {
                user_list: list,
                ..(*self).clone()
            },
        }
        .into()
    }
}

#[hook]
pub fn use_planning_poker(server_addr: &'static str) -> UsePlanningPokerReturn {
    let ws_sink = use_state(|| None);
    let state = use_reducer(State::default);

    let on_nickname_change = {
        let state = state.clone();

        Callback::from(move |event: InputEvent| {
            if let Some(input) = event.target_dyn_into::<web_sys::HtmlInputElement>() {
                state.dispatch(StateAction::Connect(input.value()));
            }
        })
    };

    let connect_callback = {
        let ws_sink = ws_sink.clone();
        let state = state.clone();

        Callback::from(move |_| {
            let state = state.clone();

            if ws_sink.borrow().is_none() {
                if let Some(sink) = connect_websocket(
                    server_addr,
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
                    let state = state.clone();

                    wasm_bindgen_futures::spawn_local(async move {
                        if let Some(nickname) = state.nickname.clone() {
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
        state: (*state).clone(),
        ws_sink,
        on_nickname_change,
        connect_callback,
        on_vote,
        on_remove_vote,
    }
}
