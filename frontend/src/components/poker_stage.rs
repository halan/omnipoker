use super::card::Card;
use crate::hooks::Stage;
use shared::Vote;
use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub stage: Stage,
    pub your_vote: Vote,
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
                    Stage::Status(status) => html! {
                        <div>
                            <div class="playingCards fourColours">
                                <ul class="table">
                                    {
                                        if props.your_vote != Vote::Null {
                                            html! {
                                                <Card vote={props.your_vote.to_string()} on_vote={props.on_remove_vote.clone()} />
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                    { for status.iter()
                                        .map(|(_, vote)| html! {
                                            <Card vote={vote.to_string()} />
                                        })
                                    }
                                </ul>
                            </div>
                        </div>
                    },
                }
            }
        </div>
    }
}
