use yew::{function_component, html, Callback, Html, Properties};

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    #[prop_or_default]
    pub back: bool,
    #[prop_or_default]
    pub vote: Option<String>,
    #[prop_or_default]
    pub player: Option<String>,
    #[prop_or_default]
    pub on_vote: Option<Callback<String>>,
}

#[function_component(Card)]
pub fn card(props: &Props) -> Html {
    let rank = match props.vote {
        Some(ref value) if value == "?" => "-",
        Some(ref value) if value == "1" => "A",
        Some(ref value) if value == "2" => "2",
        Some(ref value) if value == "3" => "3",
        Some(ref value) if value == "5" => "5",
        Some(ref value) if value == "8" => "8",
        Some(ref value) if value == "13" => "K",
        _ => "-",
    };

    let suit = match props.vote {
        Some(ref value) if value == "?" => "joker",
        Some(ref value) if value == "1" || value == "8" => "hearts",
        Some(ref value) if value == "2" || value == "13" => "spades",
        Some(ref value) if value == "3" => "diams",
        Some(ref value) if value == "5" => "clubs",
        _ => "-",
    };

    let suit_symbol = match suit {
        "spades" => "♠",
        "hearts" => "♥",
        "diams" => "♦",
        "clubs" => "♣",
        _ => "",
    };

    let on_vote = props.on_vote.clone();
    let vote = props.vote.clone();

    let onclick = on_vote.map(|callback| {
        Callback::from(move |_| {
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
                        <a
                            class={format!("card rank-{} {}", rank.to_lowercase(), suit.to_lowercase())}
                            href="#"
                            {onclick}
                        >
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
                                <div class={format!("card rank-{} {}", rank.to_lowercase(), suit.to_lowercase())}>
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
