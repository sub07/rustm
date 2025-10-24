use crate::project::Project;

pub enum View {
    ProjectOrGlobalAutomaticDetection,

    Global,
    NewProject,

    Project(Project),
    ProjectList,
}
