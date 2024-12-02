use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub user_list: Vec<String>,
    pub nickname: Option<String>,
}

#[function_component(UserList)]
pub fn user_list(props: &Props) -> Html {
    html! {
        <div>
            <h2>{ "👤 Users" }</h2>
            <ul>
            { for props.user_list.iter().map(|user| html! {
                if let Some(nickname) = &props.nickname {
                    if user == nickname {
                        <li>{ user }{ " (you)"}</li>
                    } else {
                        <li>{ user }</li>
                    }
                }
            }) }
            </ul>
        </div>
    }
}
