use crate::cli_state::CliState;
use ockam_abac::{Policies, ResourcePolicySqlxDatabase, ResourceTypePolicySqlxDatabase};
use std::sync::Arc;

impl CliState {
    pub fn policies(&self) -> Policies {
        Policies::new(
            Arc::new(ResourcePolicySqlxDatabase::new(self.database())),
            Arc::new(ResourceTypePolicySqlxDatabase::new(self.database())),
        )
    }
}
