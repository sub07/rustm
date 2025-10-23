use crate::project::Project;

pub enum State {
    Initial,

    ProjectOrGlobalAutomaticDetection,

    GlobalView,
    NewProjectView,

    ProjectView(Project),
    ProjectListView,
}

pub enum Action {
    ChooseProjectOrGlobalMode,

    OpenGlobalMode,
    OpenNewProject,

    OpenProjectMode(Project),
    OpenProjectList,
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
            (_, Action::ChooseProjectOrGlobalMode) => State::ProjectOrGlobalAutomaticDetection,
            (_, Action::OpenGlobalMode) => State::GlobalView,
            (_, Action::OpenNewProject) => State::NewProjectView,
            (_, Action::OpenProjectMode(project)) => State::ProjectView(project),
            (_, Action::OpenProjectList) => State::ProjectListView,
            _ => return,
        };
        self.0 = next_state;
    }
}
