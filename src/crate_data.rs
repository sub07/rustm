use itertools::Itertools;

use crate::crate_api;

#[derive(Debug)]
pub struct CrateData {
    pub name: String,
    pub latest_version: String,
    pub default_features: Vec<String>,
    pub features: Vec<String>,
}

impl CrateData {
    pub fn from_crate_api(dto: crate_api::dto::get_crate::Root) -> Self {
        let version = dto.versions.first().expect("Should be one version");
        let features = version.features.keys().cloned().collect_vec();
        let default_features = {
            let mut default_features = version.features.get("default").cloned().unwrap_or_default();
            default_features.retain(|feature| features.contains(feature));
            default_features
        };
        Self {
            name: dto.crate_data.name,
            latest_version: dto.crate_data.default_version,
            default_features,
            features,
        }
    }

    pub fn from_name(api: &crate_api::Client, crate_name: &str) -> anyhow::Result<Self> {
        let dto = api.get_crate(crate_name)?;
        Ok(Self::from_crate_api(dto))
    }
}
