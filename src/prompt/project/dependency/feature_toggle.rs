use inquire::MultiSelect;
use itertools::Itertools;

pub fn prompt<F: Fn(&String) -> bool>(
    features: Vec<String>,
    enabled_features_predicate: F,
) -> anyhow::Result<Vec<String>> {
    let selected_index = features
        .iter()
        .enumerate()
        .filter_map(|(index, feature)| {
            if enabled_features_predicate(feature) {
                Some(index)
            } else {
                None
            }
        })
        .collect_vec();

    #[allow(
        clippy::cast_possible_wrap,
        reason = "Casting features index with a few hundered value maximum, so no risk of wrap"
    )]
    Ok(inquire::MultiSelect::new("Enabled features", features)
        .with_default(&selected_index)
        .with_page_size(50)
        .with_scorer(&|search_input, option, string_value, idx| {
            let is_selected = selected_index.contains(&idx);
            if search_input.is_empty() {
                is_selected.then_some(idx as i64 + 1).or(Some(0))
            } else {
                MultiSelect::DEFAULT_SCORER(search_input, option, string_value, idx)
            }
        })
        .prompt()?)
}
