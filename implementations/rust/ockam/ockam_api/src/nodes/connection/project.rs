use crate::error::ApiError;
use crate::nodes::connection::{Changes, ConnectionInstanceBuilder, Instantiator};
use crate::nodes::models::secure_channel::CredentialExchangeMode;
use crate::nodes::NodeManager;
use crate::{multiaddr_to_route, try_address_to_multiaddr};

use ockam_core::{async_trait, Error};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{Match, Protocol};
use ockam_node::Context;

use std::sync::Arc;
use std::time::Duration;

/// Creates a secure connection to the project using provided credential
pub(crate) struct ProjectInstantiator {
    node_manager: Arc<NodeManager>,
    timeout: Option<Duration>,
    credential_name: Option<String>,
    identity_name: Option<String>,
    context: Arc<Context>,
}

impl ProjectInstantiator {
    pub fn new(
        context: Arc<Context>,
        node_manager: Arc<NodeManager>,
        timeout: Option<Duration>,
        credential_name: Option<String>,
        identity_name: Option<String>,
    ) -> Self {
        Self {
            context,
            node_manager,
            timeout,
            credential_name,
            identity_name,
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
        builder: &ConnectionInstanceBuilder,
        match_start: usize,
    ) -> Result<Changes, Error> {
        let (_before, project_piece, after) =
            ConnectionInstanceBuilder::extract(&builder.current_multiaddr, match_start, 1);

        let project_protocol_value = project_piece
            .first()
            .ok_or_else(|| ApiError::core("missing project protocol in multiaddr"))?;

        let project = project_protocol_value
            .cast::<Project>()
            .ok_or_else(|| ApiError::core("invalid project protocol in multiaddr"))?;

        let (project_multiaddr, project_identifier) =
            self.node_manager.resolve_project(&project).await?;

        debug!(addr = %project_multiaddr, "creating secure channel");
        let tcp = multiaddr_to_route(&project_multiaddr, &self.node_manager.tcp_transport)
            .await
            .ok_or_else(|| {
                ApiError::core(format!(
                    "Couldn't convert MultiAddr to route: project_multiaddr={project_multiaddr}"
                ))
            })?;

        let sc = self
            .node_manager
            .create_secure_channel_impl(
                tcp.route,
                Some(vec![project_identifier]),
                CredentialExchangeMode::Oneway,
                self.timeout,
                self.identity_name.clone(),
                &self.context,
                self.credential_name.clone(),
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
