use crate::error::ApiError;
use crate::nodes::connection::{Changes, Instantiator};
use crate::nodes::NodeManager;
use crate::{RemoteMultiaddrResolver, RemoteMultiaddrResolverConnection, ReverseLocalConverter};

use ockam_core::{async_trait, Error, Route};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::Context;

use crate::nodes::service::SecureChannelType;
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
        ctx: &Context,
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

        debug!(to = %project_multiaddr, identifier = %project_identifier, "creating secure channel");
        let transport_res = RemoteMultiaddrResolver::new(
            Some(node_manager.tcp_transport.clone()),
            None, // We can't connect to the project node via UDP atm
        )
        .resolve(&project_multiaddr)
        .await
        .map_err(|err| {
            ApiError::core(format!(
                "Couldn't instantiate project multiaddr. Err: {}",
                err
            ))
        })?;

        let sc = node_manager
            .create_secure_channel_internal(
                ctx,
                transport_res.route,
                &self.identifier.clone(),
                Some(vec![project_identifier]),
                None,
                self.timeout,
                SecureChannelType::KeyExchangeAndMessages,
            )
            .await?;

        // when creating a secure channel we want the route to pass through that
        // ignoring previous steps, since they will be implicit
        let mut current_multiaddr = ReverseLocalConverter::convert_address(sc.encryptor_address())?;
        current_multiaddr.try_extend(after.iter())?;

        let tcp_connection = transport_res
            .connection
            .map(|connection| match connection {
                RemoteMultiaddrResolverConnection::Tcp(tcp_connection) => Ok(tcp_connection),
                RemoteMultiaddrResolverConnection::Udp(_) => Err(ApiError::core(
                    "UDP connection can't be used to Project node",
                )),
            })
            .transpose()?;

        Ok(Changes {
            flow_control_id: Some(sc.flow_control_id().clone()),
            current_multiaddr,
            secure_channel_encryptors: vec![sc.encryptor_address().clone()],
            tcp_connection,
            udp_bind: None,
        })
    }
}
