use crate::{AbacAccessControl, Action, Env, Policies, Resource};
use core::fmt;
use core::fmt::{Debug, Formatter};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, IncomingAccessControl, RelayMessage};
use ockam_identity::{Identifier, IdentitiesAttributes};
use tracing::debug;

/// Evaluates a policy expression against an environment of attributes.
///
/// Attributes come from a pre-populated environment and are augmented
/// by subject attributes from credential data.
pub struct PolicyAccessControl {
    policies: Policies,
    identities_attributes: Arc<IdentitiesAttributes>,
    authority: Identifier,
    environment: Env,
    resource: Resource,
    action: Action,
}

/// Debug implementation writing out the resource, action and initial environment
impl Debug for PolicyAccessControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PolicyAccessControl")
            .field("resource_name", &self.resource.resource_name)
            .field("resource_type", &self.resource.resource_type)
            .field("action", &self.action)
            .field("environment", &self.environment)
            .finish()
    }
}

impl PolicyAccessControl {
    /// Create a new `PolicyAccessControl`.
    ///
    /// The policy expression is evaluated by getting subject attributes from
    /// the given authenticated storage, adding them the given environment,
    /// which may already contain other resource, action or subject attributes.
    pub fn new(
        policies: Policies,
        identities_attributes: Arc<IdentitiesAttributes>,
        authority: Identifier,
        env: Env,
        resource: Resource,
        action: Action,
    ) -> Self {
        Self {
            policies,
            identities_attributes,
            authority,
            environment: env,
            resource,
            action,
        }
    }
}

#[async_trait]
impl IncomingAccessControl for PolicyAccessControl {
    async fn is_authorized(&self, msg: &RelayMessage) -> ockam_core::Result<bool> {
        // Load the policy expression for resource and action:
        let expression = if let Some(expr) = self
            .policies
            .get_expression_for_resource(&self.resource, &self.action)
            .await?
        {
            expr
        } else {
            // If no expression exists for this resource and action, access is denied:
            debug! {
                resource = %self.resource,
                action   = %self.action,
                "no policy found; access denied"
            }
            return Ok(false);
        };

        AbacAccessControl::new(
            self.identities_attributes.clone(),
            self.authority.clone(),
            expression,
            self.environment.clone(),
        )
        .is_authorized(msg)
        .await
    }
}
