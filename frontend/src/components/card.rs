use yew::{classes, function_component, html, Callback, Html, MouseEvent, Properties};

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    #[prop_or_default]
    pub back: bool,
    #[prop_or_default]
    pub vote: Option<String>,
    #[prop_or_default]
    pub player: Option<String>,
    #[prop_or_default]
    pub your: bool,
    #[prop_or_default]
    pub on_vote: Option<Callback<String>>,
}

fn vote_to_rank(vote: &str) -> &str {
    match vote {
        "?" => "-",
        "1" => "A",
        "2" => "2",
        "3" => "3",
        "5" => "5",
        "8" => "8",
        "13" => "K",
        _ => "-",
    }
}

fn vote_to_suite(vote: &str) -> &str {
    match vote {
        "?" => "joker",
        "1" | "8" => "hearts",
        "2" | "13" => "spades",
        "3" => "diams",
        "5" => "clubs",
        _ => "-",
    }
}

fn suit_to_symbol(suit: &str) -> &str {
    match suit {
        "spades" => "♠",
        "hearts" => "♥",
        "diams" => "♦",
        "clubs" => "♣",
        _ => "",
    }
}

#[function_component(Card)]
pub fn card(props: &Props) -> Html {
    let rank = vote_to_rank(&props.vote.as_deref().unwrap_or("-"));
    let suit = vote_to_suite(&props.vote.as_deref().unwrap_or("-"));
    let suit_symbol = suit_to_symbol(suit);
    let rank_class = format!("rank-{}", rank.to_lowercase());
    let suit_class = suit.to_lowercase();
    let your_class = if props.your { "your" } else { "" };

    let on_vote = props.on_vote.clone();
    let vote = props.vote.clone();

    let onclick = on_vote.map(|callback| {
        Callback::from(move |event: MouseEvent| {
            event.prevent_default();

            if let Some(vote_value) = vote.as_deref() {
                callback.emit(vote_value.to_string());
            }
        })
    });

    html! {
        <li>
            {
                match props.on_vote {
                    Some(_) => html! {
                        <a class={classes!("card", rank_class, suit_class)} {onclick}>
                            <span class="rank">{ rank }</span>
                            <span class="suit">{ suit_symbol }</span>
                        </a>
                    },
                    None => {
                        if props.back {
                            html! {
                                <div class="card back">{ "*" }</div>
                            }
                        } else {
                            html! {
                                <div class={classes!("card", rank_class, suit_class, your_class)}>
                                    <span class="rank">{ rank }</span>
                                    <span class="suit">{ suit_symbol }</span>
                                </div>
                            }
                        }
                    }
                }
            }
            {
                if let Some(player) = &props.player {
                    html! {
                        <div class="player-nick">{ player }</div>
                    }
                } else {
                    html! {}
                }
            }
        </li>
    }
}
