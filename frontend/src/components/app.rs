use super::{
    connect_screen::ConnectScreen, hand::Hand, poker_stage::PokerStage, user_list::UserList,
};
use crate::hooks::{use_planning_poker, UsePlanningPokerReturn};

use yew::prelude::*;

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
        <div class="app">
            <h1 class="app-title">{ "Planning Poker" }</h1>
            {
            match *ws_sink {
                None => html! {
                    <ConnectScreen
                        {connect_callback}
                        {on_nickname_change}
                        nickname={(*nickname).clone()}
                    />
                },
                Some(_) => html! {
                    <>
                        <UserList user_list={(*user_list).clone()} nickname={(*nickname).clone()} />
                        <PokerStage stage={(*stage).clone()} your_vote={(*your_vote).clone()} nickname={(*nickname).clone()} {on_remove_vote} />
                        <Hand your_vote={(*your_vote).clone()} {on_vote} />
                    </>
                },
            }
        }
            <footer class="app-footer">
                {"© 2024 Planning Poker | Powered by Halan Pinheiro | "}
                <a href="http://github.com/halan/omnipoker">{ "source" }</a>
            </footer>
        </div>
    }
}
