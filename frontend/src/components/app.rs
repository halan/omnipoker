use super::{
    connect_screen::ConnectScreen, hand::Hand, poker_stage::PokerStage, user_list::UserList,
};
use crate::{
    hooks::{use_planning_poker, UsePlanningPokerReturn},
    state::Screens,
};
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    let UsePlanningPokerReturn {
        state,
        ws_sink,
        on_nickname_change,
        connect_callback,
        on_vote,
        on_remove_vote,
    } = use_planning_poker();

    html! {
        <div class="app">
            <h1 class="app-title">{ "Planning Poker" }</h1>
            {
            match (&*ws_sink, state.screen.clone()) {
                (None, _) => html! {
                    <ConnectScreen
                        {connect_callback}
                        {on_nickname_change}
                        nickname={state.nickname.clone()}
                        error_message={state.error_box.clone().unwrap_or_default()}
                    />
                },
                (Some(_), Screens::Home) => html! { <p class="loading">{"Loading..."}</p> },
                (Some(_), Screens::Game) => html! {
                    <>
                        <PokerStage
                            stage={state.stage.clone()}
                            is_rollback={state.is_rollback}
                            your_vote={state.your_vote.clone()}
                            nickname={state.nickname.clone()}
                            {on_remove_vote}
                        />
                        <UserList user_list={state.user_list.clone()} nickname={state.nickname.clone()} />
                        <Hand your_vote={state.your_vote.clone()} stage={state.stage.clone()} {on_vote} />
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
