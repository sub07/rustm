use std::process::exit;

use cargo_toml::Dependency;
use joy_error::ResultLogExt;
use log::error;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    config::Config, crate_api::Client, crate_data::CrateData, manifest_editor::ManifestEditor,
    project::Project, prompt, state::View,
};

pub enum ControlFlow {
    UpdateView(View),
    Exit,
}

macro_rules! view {
    ($v:ident$($l:tt)?) => {
        ControlFlow::UpdateView(crate::state::View::$v$($l)?)
    };
}

#[easy_ext::ext(DepdendencyExt)]
impl Dependency {
    fn default_features_enabled(&self) -> bool {
        self.detail().is_none_or(|detail| detail.default_features)
    }
}

pub fn initial() -> ControlFlow {
    match Project::current() {
        Ok(Some(project)) => view!(Project(project)),
        Ok(None) => view!(Global),
        Err(e) => {
            eprintln!("Error detecting current project: {e}");
            error!("Could not detect current project: {e}");
            view!(Global)
        }
    }
}

pub fn global() -> ControlFlow {
    use crate::prompt::global::root::SelectOption;

    let response = prompt::global::root::prompt();
    match response {
        Ok(SelectOption::NewProject) => {
            view!(NewProject)
        }
        Ok(SelectOption::ListProjects) => {
            view!(ProjectList)
        }
        Ok(SelectOption::CurrentProject(project)) => {
            view!(Project(project))
        }
        Ok(SelectOption::Exit) => ControlFlow::Exit,
        Err(e) => {
            error!("Error in global prompt: {e}");
            eprintln!("Fatal error when showing user prompt (check the logs)");
            exit(1);
        }
    }
}
pub fn project_list(config: &Config) -> ControlFlow {
    use crate::prompt::global::project_list::SelectOption;
    match prompt::global::project_list::prompt(config.projects_dir()) {
        Ok(SelectOption::SelectProject(project)) => view!(Project(project)),
        Ok(SelectOption::Back) => view!(Global),
        Err(e) => {
            error!("Error in project list prompt: {e}");
            eprintln!("Fatal error when showing user prompt (check the logs)");
            exit(1);
        }
    }
}

pub fn project(config: &Config, project: Project) -> ControlFlow {
    use crate::prompt::project::root::SelectOption;
    println!("{} [{}]", project.name, project.path.display());
    match prompt::project::root::prompt() {
        Ok(SelectOption::DependencyList) => view!(ProjectDependencyList(project)),
        Ok(SelectOption::AddCrate) => todo!(),
        Ok(SelectOption::OpenWithEditor) => {
            if let Err(e) = project.open_in_editor(config.editor_cmd()) {
                error!("Could not open project '{}' in editor: {e}", project.name);
                eprintln!("Error opening project in editor (check the logs)");
            }
            view!(Project(project))
        }
        Ok(SelectOption::GlobalMode) => view!(Global),
        Ok(SelectOption::RestoreManifest) => todo!(),
        Ok(SelectOption::Exit) => ControlFlow::Exit,
        Err(e) => {
            error!("Error in project root prompt: {e}");
            eprintln!("Fatal error when showing user prompt (check the logs)");
            exit(1);
        }
    }
}

pub fn new_project(config: &Config) -> ControlFlow {
    use crate::prompt::project::create_new;
    match create_new::prompt(config) {
        Ok(project) => {
            println!(
                "Project {} created at {}",
                project.name,
                project.path.display()
            );
            view!(Project(project))
        }
        Err(e) => {
            error!("Error creating new project: {e}");
            eprintln!("Fatal error when showing user prompt (check the logs)");
            exit(1);
        }
    }
}

pub fn project_dependency_list(project: Project, crate_api: &Client) -> ControlFlow {
    use crate::prompt::project::dependency::list::SelectOption;
    let manifest = match project.manifest() {
        Ok(manifest) => manifest,
        Err(e) => {
            error!("Could not load manifest for project {}: {e}", project.name);
            eprintln!("Fatal error when loading project manifest (check the logs)");
            exit(1);
        }
    };

    let filtered_deps = manifest
        .dependencies
        .into_par_iter()
        .filter(|(_, dep)| dep.is_crates_io())
        .map(|(name, _)| CrateData::from_name(crate_api, &name))
        .filter_map(joy_error::ResultLogExt::log_ok)
        .filter(|crate_data| !crate_data.features.is_empty())
        .map(|crate_data| crate_data.name)
        .collect::<Vec<_>>();

    match prompt::project::dependency::list::prompt(filtered_deps) {
        Ok(SelectOption::Dep { name }) => match CrateData::from_name(crate_api, &name) {
            Ok(data) => view!(ProjectDependencyDetail(project, data)),
            Err(e) => {
                println!("Could not fetch crate data");
                error!("Could not fetch '{name}' crate data: {e}");
                view!(ProjectDependencyList(project))
            }
        },
        Ok(SelectOption::Back) => view!(Project(project)),
        Err(e) => {
            error!("Error in dependency list prompt: {e}");
            eprintln!("Fatal error when showing user prompt (check the logs)");
            exit(1);
        }
    }
}

fn load_manifest_editor_or_exit(project: &Project) -> ManifestEditor {
    match ManifestEditor::from_project(project) {
        Ok(editor) => editor,
        Err(e) => {
            error!(
                "Could not load manifest editor from project {}: {e}",
                project.name
            );
            eprintln!("Fatal error when loading manifest editor (check the logs)");
            exit(1);
        }
    }
}

fn save_manifest_editor_or_exit(project: &Project, manifest_editor: &ManifestEditor) {
    if let Err(e) = manifest_editor.save() {
        error!(
            "Could not save manifest editor for project {}: {e}",
            project.name
        );
        eprintln!("Fatal error when saving manifest editor (check the logs)");
        exit(1);
    }
}

pub fn project_dependency_detail(project: Project, dep_crate_data: CrateData) -> ControlFlow {
    use crate::prompt::project::dependency::detail::SelectOption;
    println!("{} - {}", dep_crate_data.name, project.name);
    if !dep_crate_data.raw_default_features.is_empty() {
        println!("Default features: ");
        for feature in &dep_crate_data.raw_default_features {
            let is_feature_dep = !dep_crate_data.default_features.contains(feature);
            println!(
                " - {feature} {}",
                if is_feature_dep { "(Dependency)" } else { "" }
            );
        }
    }

    let dep = match project.dep(&dep_crate_data.name) {
        Ok(dep) => dep,
        Err(e) => {
            error!(
                "Could not get dependency '{}' from project '{}': {e}",
                dep_crate_data.name, project.name
            );
            eprintln!("Fatal error when loading dependency data (check the logs)");
            exit(1);
        }
    };

    match prompt::project::dependency::detail::prompt(
        dep.default_features_enabled(),
        !dep_crate_data.features.is_empty(),
    ) {
        Ok(SelectOption::Back) => view!(ProjectDependencyList(project)),
        Ok(SelectOption::Features) => {
            view!(ProjectDependencyFeatureToggle(project, dep_crate_data, dep))
        }
        Ok(SelectOption::SetDefaultFeatures(enabled)) => {
            let mut manifest_editor = load_manifest_editor_or_exit(&project);
            manifest_editor.set_dep_features(&dep_crate_data.name, None, Some(enabled));
            save_manifest_editor_or_exit(&project, &manifest_editor);
            view!(ProjectDependencyDetail(project, dep_crate_data))
        }
        Err(e) => {
            error!("Error in dependency detail prompt: {e}");
            eprintln!("Fatal error when showing user prompt (check the logs)");
            exit(1);
        }
    }
}

pub fn project_dependency_feature_toggle(
    project: Project,
    dep_crate_data: CrateData,
    manifest_dep: &Dependency,
) -> ControlFlow {
    let default_features_enabled = manifest_dep.default_features_enabled();

    let filtered_features = {
        let mut all_features = dep_crate_data.features.clone();
        all_features.retain(|f| f != "default");
        all_features
    };

    let prompt_res = prompt::project::dependency::feature_toggle::prompt(filtered_features, |f| {
        let in_default_features =
            dep_crate_data.default_features.contains(f) && default_features_enabled;
        let manually_specified = manifest_dep
            .detail()
            .is_some_and(|d| d.features.contains(f));
        in_default_features || manually_specified
    });

    match prompt_res {
        Ok(Some(newly_selected_features)) => {
            let mut manifest_editor = load_manifest_editor_or_exit(&project);

            let some_default_features_disabled = !dep_crate_data
                .default_features
                .iter()
                .all(|f| newly_selected_features.contains(f));

            let all_features_are_default = newly_selected_features
                .iter()
                .all(|f| dep_crate_data.default_features.contains(f));

            if all_features_are_default && !some_default_features_disabled {
                manifest_editor.set_dep_features(&dep_crate_data.name, Some(vec![]), Some(true));
            } else {
                manifest_editor.set_dep_features(
                    &dep_crate_data.name,
                    Some(newly_selected_features),
                    some_default_features_disabled.then_some(false),
                );
            }

            save_manifest_editor_or_exit(&project, &manifest_editor);

            view!(ProjectDependencyDetail(project, dep_crate_data))
        }
        Ok(None) => view!(ProjectDependencyDetail(project, dep_crate_data)),
        Err(e) => {
            error!("Error in dependency feature toggle prompt: {e}");
            eprintln!("Fatal error when showing user prompt (check the logs)");
            exit(1);
        }
    }
}
