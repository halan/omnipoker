use shared::{Vote, VoteStatus};
use std::rc::Rc;
use yew::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub enum Stage {
    Init,
    Result(Vec<(String, Vote)>),
    Status(Vec<(String, VoteStatus)>),
}

impl Stage {
    pub fn count_votes(&self) -> usize {
        match self {
            Stage::Status(statuses) => statuses
                .iter()
                .filter(|(_, status)| matches!(status, VoteStatus::Voted))
                .count(),
            _ => 0,
        }
    }

    pub fn your_vote_status(&self, nickname: &Option<String>) -> VoteStatus {
        match self {
            Stage::Status(statuses) => statuses
                .iter()
                .find(|(user, _)| Some(user) == nickname.as_ref())
                .map(|(_, status)| status.clone())
                .unwrap_or(VoteStatus::NotVoted),
            _ => VoteStatus::NotVoted,
        }
    }
}

pub enum StateAction {
    Result(Stage),
    Status(Stage),
    Connect(Option<String>),
    ConnectError(String),
    YourVote(Vote),
    UpdateUserList(Vec<String>),
}

#[derive(Clone, Debug)]
pub enum Screens {
    Home,
    Game,
}
#[derive(Clone)]
pub struct State {
    pub stage: Stage,
    pub nickname: Option<String>,
    pub error_box: Option<String>,
    pub your_vote: Vote,
    pub is_rollback: bool,
    pub user_list: Vec<String>,
    pub screen: Screens,
}

impl Default for State {
    fn default() -> Self {
        Self {
            stage: Stage::Init,
            nickname: None,
            error_box: None,
            your_vote: Vote::Null,
            is_rollback: false,
            user_list: Vec::new(),
            screen: Screens::Home,
        }
    }
}

impl Reducible for State {
    type Action = StateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            StateAction::Status(new_stage) => {
                let old_stage = self.stage.clone();

                Self {
                    stage: new_stage.clone(),
                    is_rollback: match (&new_stage, &old_stage) {
                        (new_stage @ Stage::Status(_), old_stage @ Stage::Status(_)) => {
                            new_stage.count_votes() < old_stage.count_votes()
                        }
                        _ => false,
                    },
                    your_vote: match new_stage.clone().your_vote_status(&self.nickname) {
                        VoteStatus::Voted => self.your_vote.clone(),
                        _ => Vote::Null,
                    },
                    error_box: None,
                    ..(*self).clone()
                }
            }
            StateAction::Result(new_stage) => Self {
                stage: new_stage,
                is_rollback: false,
                ..(*self).clone()
            },
            StateAction::Connect(nickname) => Self {
                nickname: nickname,
                ..(*self).clone()
            },
            StateAction::ConnectError(err) => Self {
                nickname: None,
                error_box: Some(err),
                screen: Screens::Home,
                ..(*self).clone()
            },
            StateAction::YourVote(vote) => Self {
                your_vote: vote,
                ..(*self).clone()
            },
            StateAction::UpdateUserList(list) => Self {
                user_list: list,
                screen: Screens::Game,
                ..(*self).clone()
            },
        }
        .into()
    }
}
