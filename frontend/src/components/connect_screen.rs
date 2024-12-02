use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub connect_callback: Callback<MouseEvent>,
    pub on_nickname_change: Callback<InputEvent>,
    pub nickname: Option<String>,
}

#[function_component(ConnectScreen)]
pub fn connect_screen(props: &Props) -> Html {
    html! {
        <div>
            <div>
                <input
                    type="text"
                    placeholder="Enter your nickname"
                    oninput={props.on_nickname_change.clone()}
                    value={props.nickname.clone()}
                />
                <button onclick={props.connect_callback.clone()}>
                    { "Connect" }
                </button>
            </div>
        </div>
    }
}
