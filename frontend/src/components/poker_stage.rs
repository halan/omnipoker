use super::card::Card;
use crate::state::Stage;
use shared::{Vote, VoteStatus};
use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub stage: Stage,
    pub is_rollback: bool,
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
                    Stage::Init => html! { <p>{"Pick a card when you're ready to vote"}</p> },
                    Stage::Result(result) => html! {
                        <div>
                            <div class="playingCards faceImages twoColours">
                                <ul class="table result">
                                { for result.iter()
                                    .map(|(nickname, result)| html! {
                                        <Card vote={result.to_string()} player={nickname.clone()} your={Some(nickname.clone()) == props.nickname.clone()} />
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

                        html! {
                            <div>
                                <div class="playingCards faceImages twoColours">
                                    <ul class={classes!("table", "status", if props.is_rollback { "rollback" } else { "" })}>
                                        { for statuses_iter
                                            .map(|(user, _)| {
                                                if Some(user) == props.nickname.as_ref() {
                                                    html! {
                                                        <Card
                                                            vote={props.your_vote.to_string()}
                                                            on_vote={props.on_remove_vote.clone()}
                                                            player={props.nickname.clone()}
                                                        />
                                                    }
                                                } else {
                                                    html! {
                                                        <Card back={true} player={user.clone()} />
                                                    }
                                                }
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
