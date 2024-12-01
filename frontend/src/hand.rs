use super::{InboundMessage, Model, Msg};
use std::rc::Rc;
use yew::{html::Scope, prelude::*};

pub fn render_cards(link: &Scope<Model>) -> Html {
    let cards = Rc::new(vec![
        ("-", "joker", "?"),
        ("2", "spades", "2"),
        ("3", "diams", "3"),
        ("5", "clubs", "5"),
        ("8", "hearts", "8"),
        ("K", "clubs", "13"),
    ]);

    html! {
        <div class="playingCards fourColours rotateHand">
            <ul class="hand">
            { for cards.iter().map(|(rank, suit, vote)| {
                let suit_symbol = match *suit {
                    "spades" => "♠",
                    "hearts" => "♥",
                    "diams" => "♦",
                    "clubs" => "♣",
                    "joker" => "",
                    _ => "",
                };
                html! {
                    <li>
                        <a
                            onclick={link.callback({
                                let vote = vote.to_string();
                                move |_| Msg::Inbound(InboundMessage::Vote { value: vote.clone() })
                            })}
                            class={format!("card rank-{} {}", rank.to_lowercase(), suit.to_lowercase())}
                            href="#"
                        >
                            <span class="rank">{ rank }</span>
                            <span class="suit">{ suit_symbol }</span>
                        </a>
                    </li>
                }
            })}
            </ul>
        </div>
    }
}
