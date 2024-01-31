use crate::CliState;
use ockam::identity::{
    IdentitiesAttributes, IdentityAttributesRepository, IdentityAttributesSqlxDatabase,
};
use std::sync::Arc;

impl CliState {
    /// Return the service managing identities attributes
    pub fn identities_attributes(&self) -> Arc<IdentitiesAttributes> {
        Arc::new(IdentitiesAttributes::new(
            self.identity_attributes_repository(),
        ))
    }

    /// The identity attributes repository cannot be accessed directly
    /// outside of the identities_attributes service
    fn identity_attributes_repository(&self) -> Arc<dyn IdentityAttributesRepository> {
        Arc::new(IdentityAttributesSqlxDatabase::new(self.database()))
    }
}
