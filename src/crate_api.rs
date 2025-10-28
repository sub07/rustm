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

    pub mod search_crates {
        #[derive(serde::Deserialize, Debug)]
        pub struct Root {
            pub crates: Vec<Crate>,
            pub meta: Meta,
        }

        #[derive(serde::Deserialize, Debug)]
        pub struct Crate {
            pub name: String,
            pub default_version: String,
            pub description: Option<String>,
            pub downloads: u64,
            pub documentation: Option<String>,
        }

        #[derive(serde::Deserialize, Debug)]
        pub struct Meta {
            pub total: u64,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Pagination {
    pub page: u64,
    pub per_page: u64,
}

impl Pagination {
    pub const fn page_count(self, total_item: u64) -> u64 {
        if total_item == 0 {
            1
        } else {
            (total_item - 1) / self.per_page + 1
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

    pub fn search_crates(
        &self,
        query: &str,
        pagination: Pagination,
    ) -> anyhow::Result<dto::search_crates::Root> {
        Ok(self
            .inner
            .get(format!("{}/crates", self.base_url))
            .query(&[
                ("q", query),
                ("sort", "relevance"),
                ("page", &pagination.page.to_string()),
                ("per_page", &pagination.per_page.to_string()),
            ])
            .send()?
            .json()?)
    }
}
