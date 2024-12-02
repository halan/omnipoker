use super::card::Card;
use crate::hooks::{use_planning_poker, Stage, UsePlanningPokerReturn};
pub use shared::{Vote, VoteStatus};

use yew::prelude::*;

const VOTES: [&'static str; 7] = ["?", "1", "2", "3", "5", "8", "13"];
const SERVER_ADDR: &str = "ws://127.0.0.1:8080/ws?mode=json";

#[function_component(App)]
pub fn app() -> Html {
    let UsePlanningPokerReturn {
        user_list,
        your_vote,
        stage,
        ws_sink,
        nickname,
        on_nickname_change,
        connect_callback,
        on_vote,
        on_remove_vote,
    } = use_planning_poker(SERVER_ADDR);

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
