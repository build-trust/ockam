use crate::error::ApiError;
use crate::nodes::connection::{Changes, Instantiator};
use crate::CliState;
use crate::{RemoteMultiaddrResolver, RemoteMultiaddrResolverConnection, ReverseLocalConverter};
use std::sync::Arc;

use ockam_core::{async_trait, Route};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::Context;

use ockam::identity::{Identifier, SecureChannelOptions, SecureChannels};
use ockam_transport_tcp::TcpTransport;
use std::time::Duration;

/// Creates a secure connection to the project using provided credential
pub(crate) struct ProjectInstantiator {
    identifier: Identifier,
    timeout: Option<Duration>,
    cli_state: CliState,
    secure_channels: Arc<SecureChannels>,
    tcp_transport: TcpTransport,
}

impl ProjectInstantiator {
    pub fn new(
        identifier: Identifier,
        timeout: Option<Duration>,
        cli_state: CliState,
        secure_channels: Arc<SecureChannels>,
        tcp_transport: TcpTransport,
    ) -> Self {
        Self {
            identifier,
            timeout,
            cli_state,
            secure_channels,
            tcp_transport,
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
        context: &Context,
        _transport_route: Route,
        extracted: (MultiAddr, MultiAddr, MultiAddr),
    ) -> Result<Changes, ockam_core::Error> {
        let (_before, project_piece, after) = extracted;

        let project_protocol_value = project_piece
            .first()
            .ok_or_else(|| ApiError::core("missing project protocol in multiaddr"))?;

        let project = project_protocol_value
            .cast::<Project>()
            .ok_or_else(|| ApiError::core("invalid project protocol in multiaddr"))?;

        let (project_multiaddr, project_identifier) = self
            .cli_state
            .projects()
            .get_project_by_name(&project)
            .await
            .map(|project| {
                (
                    project.project_multiaddr().cloned(),
                    project
                        .project_identifier()
                        .ok_or_else(|| ApiError::core("project identifier is missing")),
                )
            })?;

        let project_identifier = project_identifier?;
        let project_multiaddr = project_multiaddr?;

        debug!(to = %project_multiaddr, identifier = %project_identifier, "creating secure channel");
        let transport_res = RemoteMultiaddrResolver::new(
            Some(self.tcp_transport.clone()),
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

        debug!("create a secure channel to the project {project_identifier}");

        let options = SecureChannelOptions::new().with_authority(project_identifier);
        let options = if let Some(timeout) = self.timeout {
            options.with_timeout(timeout)
        } else {
            options
        };

        let secure_channel = self
            .secure_channels
            .create_secure_channel(
                context,
                &self.identifier.clone(),
                transport_res.route,
                options,
            )
            .await?;

        // when creating a secure channel, we want the route to pass through that
        // ignoring previous steps, since they will be implicit
        let mut current_multiaddr =
            ReverseLocalConverter::convert_address(secure_channel.encryptor_address())?;
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
            flow_control_id: Some(secure_channel.flow_control_id().clone()),
            current_multiaddr,
            secure_channel_encryptors: vec![secure_channel.encryptor_address().clone()],
            tcp_connection,
            udp_bind: None,
        })
    }
}
