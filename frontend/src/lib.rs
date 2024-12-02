use card::Card;
pub use shared::{InboundMessage, OutboundMessage, Vote, VoteStatus};
use std::borrow::Borrow;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_logger;
use ws::{connect_websocket, send_message, WebSocketSink};
use yew::{prelude::*, Renderer};

mod card;
mod ws;

enum Stage {
    Init,
    Result(Vec<(String, Vote)>),
    Status(Vec<(String, VoteStatus)>),
}

const VOTES: [&'static str; 7] = ["?", "1", "2", "3", "5", "8", "13"];

#[function_component(App)]
fn app() -> Html {
    let user_list = use_state(Vec::new);
    let your_vote = use_state(|| Vote::Null);
    let stage = use_state(|| Stage::Init);
    let ws_sink = use_state(|| None::<WebSocketSink>);
    let nickname = use_state(|| Option::None);

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
                if let Some(sink) = connect_websocket("ws://127.0.0.1:8080/ws?mode=json", {
                    let user_list = user_list.clone();
                    move |outbound| match outbound {
                        OutboundMessage::UserList(list) => {
                            user_list.set(list);
                        }

                        OutboundMessage::VotesResult(results) => {
                            stage.set(Stage::Result(results));
                            your_vote.set(Vote::Null);
                        }

                        OutboundMessage::VotesStatus(statuses) => {
                            stage.set(Stage::Status(statuses));
                        }

                        OutboundMessage::YourVote(vote) => {
                            your_vote.set(vote);
                        }
                        _ => {}
                    }
                }) {
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

    html! {
        <div>
            <h1>{ "Planning Poker" }</h1>
            {
            match *ws_sink {
                None => html! {
                    <div>
                        <div>
                            <input
                                type="text"
                                placeholder="Enter your nickname"
                                oninput={on_nickname_change}
                                value={(*nickname).clone()}
                            />
                            <button onclick={connect_callback}>
                                { "Connect" }
                            </button>
                        </div>
                    </div>
                },
                Some(_) => html! {
                    <>
                        <div>
                            <h2>{ "👤 Users" }</h2>
                            <ul>
                            { for user_list.iter().map(|user| html! {
                                if let Some(nickname) = &*nickname {
                                    if user == nickname {
                                        <li>{ user }{ " (you)"}</li>
                                    } else {
                                        <li>{ user }</li>
                                    }
                                }
                            }) }
                            </ul>
                        </div>
                        <div>
                            { match &*stage {
                                Stage::Init => html! { "..." },
                                Stage::Result(result) => html! {
                                    <div>
                                        <div class="playingCards fourColours">
                                            <ul class="table">
                                            { for result.iter()
                                                .map(|(_, result)| html! {
                                                    <Card vote={result.to_string()} />
                                                })
                                            }
                                            </ul>
                                        </div>
                                    </div>
                                },
                                Stage::Status(status) => html! {
                                    <div>
                                        <div class="playingCards fourColours">
                                            <ul class="table">
                                                {
                                                    if *your_vote != Vote::Null {
                                                        html! {
                                                            <Card vote={your_vote.to_string()} on_vote={on_remove_vote} />
                                                        }
                                                    } else {
                                                        html! {}
                                                    }
                                                }
                                                { for status.iter()
                                                    .filter(|(_, status)| *status == VoteStatus::Voted )
                                                    .filter(|(user, _)| {
                                                        if let Some(nickname) = &*nickname {
                                                            user != nickname
                                                        } else {
                                                            true
                                                        }
                                                    })
                                                    .map(|(_, _)| html! { <Card back={true} />})
                                                }
                                            </ul>
                                        </div>
                                    </div>
                                }
                            }
                        }
                        </div>
                        <div class="playingCards fourColours rotateHand">
                            <ul class="hand">
                                { for VOTES.iter()
                                    .filter(|vote| vote.to_string() != your_vote.to_string())
                                    .map(|vote| {
                                        let on_vote = on_vote.clone();
                                        html! { <Card vote={*vote} {on_vote} />}
                                    })
                                }
                            </ul>
                        </div>
                    </>
                },
            }
        }
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::default());
    Renderer::<App>::new().render();
}
