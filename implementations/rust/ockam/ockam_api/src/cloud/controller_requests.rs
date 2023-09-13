use minicbor::{Decode, Encode};
use std::str::FromStr;
use std::sync::Arc;

use ockam_core::env::{get_env, get_env_with_default, FromString};
use ockam_core::{Result, Route};
use ockam_identity::{Identifier, SecureChannels};
use ockam_multiaddr::MultiAddr;
use ockam_transport_tcp::TcpTransport;

use crate::cloud::secure_client::SecureClient;
use crate::error::ApiError;
use crate::multiaddr_to_route;

pub const OCKAM_CONTROLLER_ADDR: &str = "OCKAM_CONTROLLER_ADDR";
pub const DEFAULT_CONTROLLER_ADDRESS: &str = "/dnsaddr/orchestrator.ockam.io/tcp/6252/service/api";

/// If it's present, its contents will be used and will have priority over the contents
/// from ./static/controller.id.
/// How to use: when running a command that spawns a background node or use an embedded node
/// add the env variable. `OCKAM_CONTROLLER_IDENTITY_ID={identity.id-contents} ockam ...`
pub(crate) const OCKAM_CONTROLLER_IDENTITY_ID: &str = "OCKAM_CONTROLLER_IDENTITY_ID";

/// A default timeout in seconds
pub const ORCHESTRATOR_RESTART_TIMEOUT: u64 = 180;

/// Total time in milliseconds to wait for Orchestrator long-running operations to complete
pub const ORCHESTRATOR_AWAIT_TIMEOUT_MS: usize = 60 * 10 * 1000;

/// A wrapper around a cloud request with extra fields.
#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CloudRequestWrapper<T> {
    #[b(1)] pub req: T,
}

impl<T> CloudRequestWrapper<T> {
    pub fn new(req: T) -> Self {
        Self { req }
    }
}

impl SecureClient {
    pub async fn controller(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        caller_identifier: Identifier,
    ) -> Result<SecureClient> {
        let controller_route = Self::controller_route(&tcp_transport).await?;
        let controller_identifier = Self::load_controller_identifier()?;

        Ok(SecureClient::new(
            secure_channels,
            controller_route,
            controller_identifier,
            caller_identifier,
        ))
    }

    /// Load controller identity id from file.
    /// If the env var `OCKAM_CONTROLLER_IDENTITY_ID` is set, that will be used to
    /// load the identifier instead of the file.
    pub fn load_controller_identifier() -> Result<Identifier> {
        if let Ok(Some(idt)) = get_env::<Identifier>(OCKAM_CONTROLLER_IDENTITY_ID) {
            trace!(idt = %idt, "Read controller identifier from env");
            return Ok(idt);
        }
        Identifier::from_str(include_str!("../../static/controller.id"))
    }

    pub fn controller_multiaddr() -> MultiAddr {
        let default_addr = MultiAddr::from_string(DEFAULT_CONTROLLER_ADDRESS)
            .unwrap_or_else(|_| panic!("invalid Controller address: {DEFAULT_CONTROLLER_ADDRESS}"));
        get_env_with_default::<MultiAddr>(OCKAM_CONTROLLER_ADDR, default_addr).unwrap()
    }

    async fn controller_route(tcp_transport: &TcpTransport) -> Result<Route> {
        let controller_multiaddr = Self::controller_multiaddr();
        Ok(multiaddr_to_route(&controller_multiaddr, &tcp_transport)
            .await
            .ok_or_else(|| {
                ApiError::core(format!(
                    "Couldn't convert MultiAddr to route: controller_multiaddr={controller_multiaddr}"
                ))
            })?.route)
    }

}
