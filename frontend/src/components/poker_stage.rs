use super::card::Card;
use crate::hooks::Stage;
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
        <div>
            {
                match &props.stage {
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
                    Stage::Status(statuses) => {
                        let statuses = statuses
                            .iter()
                            .filter(|(_, status)| *status == VoteStatus::Voted );

                        let you_voted = props.your_vote != Vote::Null &&
                            statuses.clone().any(|(user, _)| user == props.nickname.as_ref().unwrap());

                        html! {
                            <div>
                                <div class="playingCards fourColours">
                                    <ul class="table">
                                        {
                                            if you_voted {
                                                html! {
                                                    <Card vote={props.your_vote.to_string()} on_vote={props.on_remove_vote.clone()} />
                                                }
                                            } else {
                                                html! {}
                                            }
                                        }
                                        { for statuses
                                            .filter(|(user, _)| {
                                                if let Some(nickname) = &props.nickname {
                                                    user != nickname
                                                } else {
                                                    true
                                                }
                                            })
                                            .map(|(_, _)| html! {
                                                <Card back={true} />
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
