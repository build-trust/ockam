use crate::traits::PolicyStorage;
use crate::types::{Action, Resource};
use crate::AbacAccessControl;
use crate::{Env, Expr};
use core::fmt;
use core::fmt::{Debug, Formatter};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::format;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, RelayMessage};
use ockam_core::{IncomingAccessControl, Result};
use ockam_identity::authenticated_storage::IdentityAttributeStorage;
use tracing as log;

/// Evaluates a policy expression against an environment of attributes.
///
/// Attributes come from a pre-populated environment and are augmented
/// by subject attributes from credential data.
pub struct PolicyAccessControl {
    resource: Resource,
    action: Action,
    policies: Arc<dyn PolicyStorage>,
    attributes: Arc<dyn IdentityAttributeStorage>,
    environment: Env,
}

/// Debug implementation writing out the resource, action and initial environment
impl Debug for PolicyAccessControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let resource = &self.resource;
        let action = &self.resource;
        let environment = &self.environment;
        f.write_str(format!("resource {resource:?}").as_str())?;
        f.write_str(format!("action {action:?}").as_str())?;
        f.write_str(format!("environment {environment:?}").as_str())
    }
}

impl PolicyAccessControl {
    /// Create a new `PolicyAccessControl`.
    ///
    /// The policy expression is evaluated by getting subject attributes from
    /// the given authenticated storage, adding them the given environment,
    /// which may already contain other resource, action or subject attributes.
    pub fn new(
        policies: Arc<dyn PolicyStorage>,
        store: Arc<dyn IdentityAttributeStorage>,
        r: Resource,
        a: Action,
        env: Env,
    ) -> Self {
        Self {
            resource: r,
            action: a,
            policies,
            attributes: store,
            environment: env,
        }
    }
}

#[async_trait]
impl IncomingAccessControl for PolicyAccessControl {
    async fn is_authorized(&self, msg: &RelayMessage) -> Result<bool> {
        // Load the policy expression for resource and action:
        let expr = if let Some(expr) = self
            .policies
            .get_policy(&self.resource, &self.action)
            .await?
        {
            if let Expr::Bool(b) = expr {
                // If the policy is a constant there is no need to populate
                // the environment or look for message metadata.
                return Ok(b);
            } else {
                expr
            }
        } else {
            // If no policy exists for this resource and action access is denied:
            log::debug! {
                resource = %self.resource,
                action   = %self.action,
                "no policy found; access denied"
            }
            return Ok(false);
        };

        AbacAccessControl::new(self.attributes.clone(), expr, self.environment.clone())
            .is_authorized(msg)
            .await
    }
}
