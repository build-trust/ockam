use crate::abac::Abac;
use crate::policy::{IncomingPolicyAccessControl, OutgoingPolicyAccessControl};
use crate::{Action, Env, Policies, Resource};
use core::fmt;
use core::fmt::{Debug, Formatter};
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, DenyAll, Result};
use ockam_identity::{Identifier, IdentitiesAttributes};
use ockam_node::Context;
use tracing::debug;

/// Evaluates a policy expression against an environment of attributes.
///
/// Attributes come from a pre-populated environment and are augmented
/// by subject attributes from credential data.
#[derive(Clone)]
pub struct PolicyAccessControl {
    pub(super) abac: Abac,
    pub(super) policies: Policies,
    pub(super) resource: Resource,
    pub(super) action: Action,
}

/// Debug implementation writing out the resource, action and initial environment
impl Debug for PolicyAccessControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PolicyAccessControl")
            .field("resource_name", &self.resource.resource_name)
            .field("resource_type", &self.resource.resource_type)
            .field("action", &self.action)
            .field("abac", &self.abac)
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
        authority: Option<Identifier>,
        env: Env,
        resource: Resource,
        action: Action,
    ) -> Self {
        let abac = Abac::new(identities_attributes, authority, env);
        Self {
            abac,
            policies,
            resource,
            action,
        }
    }

    pub fn create_incoming(&self) -> IncomingPolicyAccessControl {
        IncomingPolicyAccessControl {
            policy_access_control: self.clone(),
        }
    }

    pub async fn create_outgoing(&self, ctx: &Context) -> Result<OutgoingPolicyAccessControl> {
        let ctx = ctx
            .new_detached(
                Address::random_tagged("OutgoingPolicyAbac"),
                DenyAll,
                DenyAll,
            )
            .await?;

        Ok(OutgoingPolicyAccessControl {
            ctx,
            policy_access_control: self.clone(),
        })
    }

    pub async fn is_identity_authorized(&self, identifier: &Identifier) -> Result<bool> {
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

        self.abac
            .is_identity_authorized(identifier, &expression)
            .await
    }
}
