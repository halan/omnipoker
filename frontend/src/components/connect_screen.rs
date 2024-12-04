use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub connect_callback: Callback<SubmitEvent>,
    pub on_nickname_change: Callback<InputEvent>,
    pub nickname: Option<String>,
    pub error_message: Option<String>,
}

#[function_component(ConnectScreen)]
pub fn connect_screen(props: &Props) -> Html {
    html! {
        <div class="connect-screen">
            <div>
                <form onsubmit={props.connect_callback.clone()}>
                    <input
                        type="text"
                        placeholder="Enter your nickname"
                        oninput={props.on_nickname_change.clone()}
                        value={props.nickname.clone()}
                    />
                    <p>{ props.error_message.clone() }</p>
                </form>
            </div>
        </div>
    }
}
