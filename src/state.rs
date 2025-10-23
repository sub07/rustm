use crate::{
    config::{Config, RawConfig},
    project::Project,
};

pub enum State {
    Initial,

    ProjectOrGlobalAutomaticDetection(Config),

    ConfigSetupView(RawConfig),

    GlobalView(Config),
    NewProjectView(Config),

    ProjectView(Config, Project),
    ProjectListView(Config),
}

pub enum Action {
    SetupConfig(RawConfig),
    EndSetup,

    ChooseProjectOrGlobalMode(Config),

    OpenGlobalMode(Config),
    OpenNewProject(Config),

    OpenProjectMode(Config, Project),
    OpenProjectList(Config),
}

pub struct ViewStateMachine(State);

impl ViewStateMachine {
    pub const fn new() -> Self {
        Self(State::Initial)
    }

    pub const fn state(&self) -> &State {
        &self.0
    }

    #[allow(
        clippy::match_same_arms,
        reason = "For better maintenability we do not merge match arms"
    )]
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
            (State::GlobalView(_), Action::OpenProjectList(config)) => {
                State::ProjectListView(config)
            }
            (State::GlobalView(_), Action::OpenProjectMode(config, project)) => {
                State::ProjectView(config, project)
            }
            (State::GlobalView(_), Action::OpenNewProject(config)) => State::NewProjectView(config),
            (State::ProjectListView(_), Action::OpenProjectMode(config, project)) => {
                State::ProjectView(config, project)
            }
            (State::NewProjectView(_), Action::OpenProjectMode(config, project)) => {
                State::ProjectView(config, project)
            }
            _ => return,
        };
        self.0 = next_state;
    }
}
