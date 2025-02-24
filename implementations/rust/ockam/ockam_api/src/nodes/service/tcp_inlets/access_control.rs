use crate::nodes::service::tcp_inlets::session_replacer::InletParameters;
use crate::nodes::NodeManager;
use crate::ApiError;
use ockam_abac::Action;
use ockam_core::{IncomingAccessControl, OutgoingAccessControl};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::sync::Arc;

pub(in crate::nodes::service::tcp_inlets) async fn inlet_access_control(
    context: &Context,
    node_manager: &NodeManager,
    parameters: &InletParameters,
    original_multi_addr: Option<&MultiAddr>,
) -> ockam_core::Result<(
    Arc<dyn IncomingAccessControl>,
    Arc<dyn OutgoingAccessControl>,
)> {
    let authority = {
        if let Some(original_multi_addr) = original_multi_addr {
            if let Some(p) = original_multi_addr.first() {
                if let Some(p) = p.cast::<Project>() {
                    if let Ok(p) = node_manager
                        .cli_state
                        .projects()
                        .get_project_by_name(&p)
                        .await
                    {
                        Some(
                            p.authority_identifier()
                                .ok_or_else(|| ApiError::core("no authority identifier"))?,
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    .or(node_manager.project_authority());

    node_manager
        .access_control(
            context,
            authority,
            parameters.resource.clone(),
            Action::HandleMessage,
            parameters.policy_expression.clone(),
        )
        .await
}
