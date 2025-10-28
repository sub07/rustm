use std::time::Duration;

const CRATES_API_BASE_URL: &str = "https://crates.io/api/v1";

#[derive(Clone)]
pub struct Client {
    inner: reqwest::blocking::Client,
    base_url: String,
}

pub mod dto {
    pub mod get_crate {
        use std::collections::HashMap;

        #[derive(serde::Deserialize, Debug)]
        pub struct Root {
            #[serde(rename = "crate")]
            pub crate_data: Crate,
            pub versions: Vec<Version>,
        }

        #[derive(serde::Deserialize, Debug)]
        pub struct Crate {
            pub default_version: String,
            pub name: String,
        }

        #[derive(serde::Deserialize, Debug)]
        pub struct Version {
            pub features: HashMap<String, Vec<String>>,
        }
    }
}

impl Client {
    pub fn new() -> anyhow::Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("rustm (pardomarius@gmail.com)")
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            inner: client,
            base_url: CRATES_API_BASE_URL.to_string(),
        })
    }

    pub fn get_crate(&self, crate_name: &str) -> anyhow::Result<dto::get_crate::Root> {
        Ok(self
            .inner
            .get(format!("{}/crates/{}", self.base_url, crate_name))
            .query(&[("include", "default_version")])
            .send()?
            .json()?)
    }
}
