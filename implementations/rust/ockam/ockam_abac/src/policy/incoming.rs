use crate::abac::Abac;
use crate::PolicyAccessControl;
use core::fmt::Debug;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use ockam_core::{async_trait, IncomingAccessControl, RelayMessage};
use tracing::debug;

#[derive(Debug)] // FIXME: impl debug
pub struct IncomingPolicyAccessControl {
    pub(super) policy_access_control: PolicyAccessControl,
}

#[async_trait]
impl IncomingAccessControl for IncomingPolicyAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
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

        let identifier = match Abac::get_incoming_identifier(relay_msg) {
            Some(identifier) => identifier,
            None => {
                debug! {
                    policy = %expression,
                    "identity identifier not found; access denied"
                }

                return Ok(false);
            }
        };

        self.policy_access_control
            .abac
            .is_identity_authorized(&identifier, &expression)
            .await
    }
}
