use crate::{
    config::{Config, RawConfig},
    project::Project,
};

pub enum State {
    Initial,

    ProjectOrGlobalAutomaticDetection(Config),

    ConfigSetupView(RawConfig),

    GlobalView(Config),

    ProjectView(Config, Project),
    ProjectListView,
}

pub enum Action {
    SetupConfig(RawConfig),
    EndSetup,

    ChooseProjectOrGlobalMode(Config),

    OpenGlobalMode(Config),

    OpenProjectMode(Config, Project),
    OpenProjectList,
}

pub struct ViewStateMachine(State);

impl ViewStateMachine {
    pub fn new() -> Self {
        Self(State::Initial)
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn consume(&mut self, input: Action) {
        let next_state = match (&self.0, input) {
            (State::Initial, Action::SetupConfig(raw_config)) => State::ConfigSetupView(raw_config),
            (State::Initial, Action::OpenProjectMode(config, project)) => {
                State::ProjectView(config, project)
            }
            (State::Initial, Action::OpenGlobalMode(config)) => State::GlobalView(config),
            (State::Initial, Action::ChooseProjectOrGlobalMode(config)) => {
                State::ProjectOrGlobalAutomaticDetection(config)
            }
            (
                State::ProjectOrGlobalAutomaticDetection(_),
                Action::OpenProjectMode(config, project),
            ) => State::ProjectView(config, project),
            (State::ProjectOrGlobalAutomaticDetection(_), Action::OpenGlobalMode(config)) => {
                State::GlobalView(config)
            }
            (State::ConfigSetupView(_), Action::EndSetup) => State::Initial,
            (State::ProjectView(_, _), Action::OpenGlobalMode(config)) => State::GlobalView(config),
            (State::ProjectView(_, _), Action::OpenProjectList) => State::ProjectListView,
            (State::GlobalView(_), Action::OpenProjectMode(config, project)) => {
                State::ProjectView(config, project)
            }
            _ => return,
        };
        self.0 = next_state;
    }
}
