use super::card::Card;
use crate::hooks::Stage;
use _Props::nickname;
use shared::{Vote, VoteStatus};
use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub stage: Stage,
    pub your_vote: Vote,
    pub nickname: Option<String>,
    pub on_remove_vote: Callback<String>,
}

#[function_component(PokerStage)]
pub fn poker_stage(props: &Props) -> Html {
    html! {
        <div class="stage">
            {
                match &props.stage {
                    Stage::Init => html! { "..." },
                    Stage::Result(result) => html! {
                        <div>
                            <div class="playingCards fourColours">
                                <ul class="table">
                                { for result.iter()
                                    .map(|(_nickname, result)| html! {
                                        <Card vote={result.to_string()} player={_nickname.clone()} />
                                    })
                                }
                                </ul>
                            </div>
                        </div>
                    },
                    Stage::Status(statuses) => {
                        let statuses_iter = statuses
                            .iter()
                            .filter(|(_, status)| *status == VoteStatus::Voted );

                        let you_voted = props.your_vote != Vote::Null &&
                            statuses_iter.clone().any(|(user, _)| user == props.nickname.as_ref().unwrap());

                        html! {
                            <div>
                                <div class="playingCards fourColours">
                                    <ul class="table">
                                        {
                                            if you_voted {
                                                html! {
                                                    <Card
                                                        vote={props.your_vote.to_string()}
                                                        on_vote={props.on_remove_vote.clone()}
                                                        player={props.nickname.clone()} />
                                                }
                                            } else {
                                                html! {}
                                            }
                                        }
                                        { for statuses_iter
                                            .filter(|(user, _)| {
                                                if let Some(_nickname) = &props.nickname {
                                                    user != _nickname
                                                } else {
                                                    true
                                                }
                                            })
                                            .map(|(user, _)| html! {
                                                <Card back={true} player={user.clone()} />
                                            })
                                        }
                                    </ul>
                                </div>
                            </div>
                        }
                    },
                }
            }
        </div>
    }
}
