use crate::abac::Abac;
use crate::PolicyAccessControl;
use core::fmt::{Debug, Formatter};
use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, RelayMessage};
use ockam_core::{OutgoingAccessControl, Result};
use ockam_node::ContextRouter;
use tracing::debug;

pub struct OutgoingPolicyAccessControl {
    pub(super) ctx: ContextRouter,
    pub(super) policy_access_control: PolicyAccessControl,
}

impl Debug for OutgoingPolicyAccessControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OutgoingPolicyAccessControl")
            .field("policy_access_control", &self.policy_access_control)
            .finish()
    }
}

#[async_trait]
impl OutgoingAccessControl for OutgoingPolicyAccessControl {
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
            debug!("found the policy {expr:?} to be used for outgoing access control");
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

        let outgoing_info = Abac::get_outgoing_info(&self.ctx, relay_msg)?;

        self.policy_access_control
            .abac
            .is_authorized(
                outgoing_info.receiver_identifier.as_ref(),
                outgoing_info.message_sent_to_the_same_node,
                &expression,
            )
            .await
    }
}
