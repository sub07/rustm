use log::error;

use crate::{
    crate_api::{self, Pagination},
    crate_data::CrateData,
    project::Project,
    prompt,
    state::View,
};

use super::ControlFlow;

/// Convenience constructor (avoids depending on the parent view! macro)
fn view(v: View) -> ControlFlow {
    ControlFlow::Continue(Box::new(v))
}

fn view_project(project: Project) -> ControlFlow {
    view(View::Project(project))
}

fn view_project_feature_toggle(
    project: Project,
    crate_data: CrateData,
    dep: cargo_toml::Dependency,
) -> ControlFlow {
    view(View::ProjectDependencyFeatureToggle(
        project, crate_data, dep,
    ))
}

/// Prompt the user for a search query. Returns:
/// - Ok(Some(query)) on user input
/// - Ok(None) if user cancelled (we should go back)
/// - Err on fatal prompt error
fn prompt_search_query() -> anyhow::Result<Option<String>> {
    prompt::project::dependency::new::prompt_search_query()
}

/// Fetch a page of crates.io search results. Logs and returns None on error.
fn fetch_search_page(
    client: &crate_api::Client,
    query: &str,
    pagination: Pagination,
) -> Option<crate_api::dto::search_crates::Root> {
    match client.search_crates(query, pagination) {
        Ok(res) => Some(res),
        Err(e) => {
            error!("Error searching crates for '{query}': {e:?}");
            eprintln!("Could not search for crates");
            None
        }
    }
}

const fn page_count(pagination: Pagination, total: u64) -> u64 {
    if total == 0 {
        0
    } else {
        total.div_ceil(pagination.per_page)
    }
}

fn to_crate_options(
    res: crate_api::dto::search_crates::Root,
) -> Vec<prompt::project::dependency::new::CrateOption> {
    res.crates
        .into_iter()
        .map(|c| prompt::project::dependency::new::CrateOption {
            name: c.name,
            description: c.description.unwrap_or_default(),
            version: c.default_version,
            downloads: c.downloads,
        })
        .collect()
}

/// Add the dependency to the manifest, handle duplicates, and optionally transition to feature toggle.
fn add_dependency_and_maybe_edit_features(
    project: Project,
    crate_api: &crate_api::Client,
    name: &str,
    version: &str,
) -> ControlFlow {
    if project.dep(name).is_ok() {
        println!("Dependency '{name}' is already present in the project");
        return view_project(project);
    }

    let mut manifest_editor = super::load_manifest_editor_or_exit(&project);
    manifest_editor.ensure_dep_section_exists();
    manifest_editor.add_dep(name, version);

    if let Err(e) = manifest_editor.save() {
        error!(
            "Could not add dependency to project '{}': {e}",
            project.name
        );
        eprintln!("Could not add dependency");
        return view_project(project);
    }

    println!("Dependency {name} = {version} added successfully");

    let Ok(crate_data) =
        CrateData::from_name(crate_api, name).inspect_err(|e| error!("{e}"))
    else {
        return view_project(project);
    };

    if crate_data.features.is_empty() {
        return view_project(project);
    }

    let edit_now = prompt::project::dependency::new::prompt_confirmation_for_editing_feature()
        .inspect_err(|e| error!("{e}"))
        .unwrap_or(false);

    if !edit_now {
        return view_project(project);
    }

    let Ok(local_dep) = project.dep(name).inspect_err(|e| error!("{e}")) else {
        eprintln!("Could not re-load dependency after adding it");
        return view_project(project);
    };

    view_project_feature_toggle(project, crate_data, local_dep)
}

pub fn project_add_dependency(project: Project, crate_api: &crate_api::Client) -> ControlFlow {
    use prompt::project::dependency::new::PaginationOption;

    const DEFAULT_PAGINATION: Pagination = Pagination {
        page: 1,
        per_page: 10,
    };

    let search_query = match prompt_search_query() {
        Ok(Some(q)) => q,
        Ok(None) => return view_project(project),
        Err(e) => {
            error!("Error in add dependency search prompt: {e}");
            eprintln!("Fatal error when showing user prompt (check the logs)");
            return ControlFlow::Exit;
        }
    };

    let mut pagination = DEFAULT_PAGINATION;

    loop {
        let Some(search_res) = fetch_search_page(crate_api, &search_query, pagination) else {
            return view_project(project);
        };

        if search_res.crates.is_empty() {
            println!("No crates found for query '{search_query}'");
            return view_project(project);
        }

        let total_items = search_res.meta.total;
        let total_pages = page_count(pagination, total_items);

        println!(
            "Page {}/{} Total: {}",
            pagination.page,
            total_pages.max(1),
            total_items
        );

        let crate_options = to_crate_options(search_res);

        let selection = prompt::project::dependency::new::prompt_paginated_select(
            crate_options,
            pagination.page == 1,
            total_pages == 0 || pagination.page >= total_pages,
        );

        match selection {
            Ok(PaginationOption::Crate(c)) => {
                return add_dependency_and_maybe_edit_features(
                    project, crate_api, &c.name, &c.version,
                );
            }
            Ok(PaginationOption::Back) => return view_project(project),
            Ok(PaginationOption::NextPage) => {
                if total_pages > 0 {
                    pagination.page = (pagination.page + 1).min(total_pages);
                }
            }
            Ok(PaginationOption::PreviousPage) => {
                if pagination.page > 1 {
                    pagination.page -= 1;
                }
            }
            Err(e) => {
                error!("Error in add dependency paginated select prompt: {e}");
                eprintln!("Fatal error when showing user prompt (check the logs)");
                return ControlFlow::Exit;
            }
        }
    }
}
