use crate::PolicyAccessControl;
use core::fmt::{Debug, Formatter};
use ockam_core::Result;
use ockam_identity::Identifier;
use tracing::debug;

pub struct ManualPolicyAccessControl {
    pub(super) policy_access_control: PolicyAccessControl,
}

impl Debug for ManualPolicyAccessControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ManualPolicyAccessControl")
            .field("policy_access_control", &self.policy_access_control)
            .finish()
    }
}

impl ManualPolicyAccessControl {
    pub async fn is_identity_authorized(&self, identifier: &Identifier) -> Result<bool> {
        // Load the policy expression for resource and action:
        let expression = if let Some(expr) = self
            .policy_access_control
            .policies
            .get_expression_for_resource(
                &self.policy_access_control.resource,
                &self.policy_access_control.action,
            )
            .await?
        {
            expr
        } else {
            // If no expression exists for this resource and action, access is denied:
            debug! {
                resource = %self.policy_access_control.resource,
                action   = %self.policy_access_control.action,
                "no policy found; access denied"
            }
            return Ok(false);
        };

        self.policy_access_control
            .abac
            .is_identity_authorized(identifier, &expression)
            .await
    }
}
