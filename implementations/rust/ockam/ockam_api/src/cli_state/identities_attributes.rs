use crate::CliState;
use ockam::identity::{
    IdentitiesAttributes, IdentityAttributesRepository, IdentityAttributesSqlxDatabase,
};
use std::sync::Arc;

impl CliState {
    /// Return the service managing identities attributes
    pub fn identities_attributes(&self, node_name: &str) -> Arc<IdentitiesAttributes> {
        Arc::new(IdentitiesAttributes::new(
            self.identity_attributes_repository(node_name),
        ))
    }

    /// The identity attributes repository cannot be accessed directly
    /// outside of the identities_attributes service
    fn identity_attributes_repository(
        &self,
        node_name: &str,
    ) -> Arc<dyn IdentityAttributesRepository> {
        IdentityAttributesSqlxDatabase::make_repository(self.database(), node_name)
    }
}
