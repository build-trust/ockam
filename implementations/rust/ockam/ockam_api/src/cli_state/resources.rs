use crate::CliState;
use ockam_abac::{Resources, ResourcesSqlxDatabase};

impl CliState {
    pub fn resources(&self, node_name: &str) -> Resources {
        Resources::new(ResourcesSqlxDatabase::make_repository(
            self.database(),
            node_name,
        ))
    }
}
