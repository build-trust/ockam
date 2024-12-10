use crate::cli_state::CliState;
use ockam_abac::{Policies, ResourcePolicySqlxDatabase, ResourceTypePolicySqlxDatabase};

impl CliState {
    pub fn policies(&self, node_name: &str) -> Policies {
        Policies::new(
            ResourcePolicySqlxDatabase::make_repository(self.database(), node_name),
            ResourceTypePolicySqlxDatabase::make_repository(self.database(), node_name),
        )
    }
}
