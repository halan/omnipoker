use super::card::Card;
use crate::state::Stage;
use shared::{UserStatus, Vote};
use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub your_vote: Vote,
    pub stage: Stage,
    pub on_vote: Callback<String>,
    pub on_away_back: Callback<String>,
    pub on_set_away: Callback<MouseEvent>,
    pub your_status: UserStatus,
}

const VOTES: [&str; 7] = ["?", "1", "2", "3", "5", "8", "13"];

#[function_component(Hand)]
pub fn hand(props: &Props) -> Html {
    html! {
        <>
            <div class="playingCards twoColours rotateHand">
                <ul class="hand">
                    { for VOTES.iter()
                        .map(|vote| {
                            let on_vote = props.on_vote.clone();
                            let on_away_back = props.on_away_back.clone();

                            match (&props.stage, &props.your_status) {
                                (_, UserStatus::Away) => html! { <Card back={true} on_vote={on_away_back} /> },
                                // Restart the game
                                (Stage::Result(_), _) => html! { <Card vote={*vote} on_vote={on_vote} /> },
                                _ => if vote.to_string() != props.your_vote.to_string() {
                                    html! { <Card vote={*vote} on_vote={on_vote} /> }
                                } else {
                                    html! { <li/> }
                                },
                            }
                        })
                    }
                </ul>
            </div>
            {
                match &props.your_status {
                    UserStatus::Active => html!{
                        <p class="actions"><button onclick={props.on_set_away.clone()}>{ "Set away..." }</button></p>
                    },
                    _ => html!{},
                }
            }
        </>
    }
}
