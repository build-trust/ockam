use crate::error::ApiError;
use crate::nodes::connection::{Changes, Instantiator};
use crate::nodes::NodeManager;
use crate::{multiaddr_to_route, try_address_to_multiaddr};
use std::sync::Arc;

use ockam_core::{async_trait, Error, Route};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::Context;

use ockam::identity::Identifier;
use std::time::Duration;

/// Creates a secure connection to the project using provided credential
pub(crate) struct ProjectInstantiator {
    identifier: Identifier,
    timeout: Option<Duration>,
}

impl ProjectInstantiator {
    pub fn new(identifier: Identifier, timeout: Option<Duration>) -> Self {
        Self {
            identifier,
            timeout,
        }
    }
}

#[async_trait]
impl Instantiator for ProjectInstantiator {
    fn matches(&self) -> Vec<Match> {
        vec![Project::CODE.into()]
    }

    async fn instantiate(
        &self,
        ctx: Arc<Context>,
        node_manager: &NodeManager,
        _transport_route: Route,
        extracted: (MultiAddr, MultiAddr, MultiAddr),
    ) -> Result<Changes, Error> {
        let (_before, project_piece, after) = extracted;

        let project_protocol_value = project_piece
            .first()
            .ok_or_else(|| ApiError::core("missing project protocol in multiaddr"))?;

        let project = project_protocol_value
            .cast::<Project>()
            .ok_or_else(|| ApiError::core("invalid project protocol in multiaddr"))?;

        let (project_multiaddr, project_identifier) =
            node_manager.resolve_project(&project).await?;

        debug!(addr = %project_multiaddr, "creating secure channel");
        let tcp = multiaddr_to_route(&project_multiaddr, &node_manager.tcp_transport)
            .await
            .ok_or_else(|| {
                ApiError::core(format!(
                    "Couldn't convert MultiAddr to route: project_multiaddr={project_multiaddr}"
                ))
            })?;

        debug!("create a secure channel to the project {project_identifier}");
        let sc = node_manager
            .create_secure_channel_internal(
                &ctx,
                tcp.route,
                &self.identifier.clone(),
                Some(vec![project_identifier]),
                self.timeout,
            )
            .await?;

        // when creating a secure channel we want the route to pass through that
        // ignoring previous steps, since they will be implicit
        let mut current_multiaddr = try_address_to_multiaddr(sc.encryptor_address()).unwrap();
        current_multiaddr.try_extend(after.iter())?;

        Ok(Changes {
            flow_control_id: Some(sc.flow_control_id().clone()),
            current_multiaddr,
            secure_channel_encryptors: vec![sc.encryptor_address().clone()],
            tcp_connection: tcp.tcp_connection,
        })
    }
}
