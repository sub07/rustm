use cargo_toml::Dependency;

use crate::{crate_data::CrateData, project::Project};

pub enum View {
    Initial,

    Global,
    NewProject,
    ProjectList,

    Project(Project),
    ProjectAddDependency(Project),
    ProjectDependencyList(Project),
    ProjectDependencyDetail(Project, CrateData),
    ProjectDependencyFeatureToggle(Project, CrateData, Dependency),
}
