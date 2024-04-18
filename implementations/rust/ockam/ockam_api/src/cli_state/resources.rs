use crate::CliState;
use ockam_abac::{Resources, ResourcesSqlxDatabase};
use std::sync::Arc;

impl CliState {
    pub fn resources(&self, node_name: &str) -> Resources {
        Resources::new(Arc::new(ResourcesSqlxDatabase::new(
            self.database(),
            node_name,
        )))
    }
}
