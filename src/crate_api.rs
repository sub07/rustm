use std::{rc::Rc, time::Duration};

use crates_io_api::SyncClient;

#[derive(Clone)]
struct CrateApi {
    client: Rc<SyncClient>,
}

impl CrateApi {
    pub fn new() -> anyhow::Result<Self> {
        let client = SyncClient::new("rustm", Duration::from_secs(1))?;
        Ok(Self {
            client: Rc::new(client),
        })
    }
}
