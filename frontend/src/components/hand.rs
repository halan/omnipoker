use super::card::Card;
use shared::Vote;
use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub your_vote: Vote,
    pub on_vote: Callback<String>,
}

const VOTES: [&str; 7] = ["?", "1", "2", "3", "5", "8", "13"];

#[function_component(Hand)]
pub fn hand(props: &Props) -> Html {
    html! {
        <div class="playingCards fourColours rotateHand">
            <ul class="hand">
                { for VOTES.iter()
                    .filter(|vote| vote.to_string() != props.your_vote.to_string())
                    .map(|vote| {
                        let on_vote = props.on_vote.clone();
                        html! { <Card vote={*vote} {on_vote} />}
                    })
                }
            </ul>
        </div>
    }
}
