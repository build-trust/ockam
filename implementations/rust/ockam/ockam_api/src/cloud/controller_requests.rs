use minicbor::{Decode, Encode};

use ockam_core::env::{get_env_with_default, FromString};
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::MultiAddr;

pub const OCKAM_CONTROLLER_ADDR: &str = "OCKAM_CONTROLLER_ADDR";
pub const DEFAULT_CONTROLLER_ADDRESS: &str = "/dnsaddr/orchestrator.ockam.io/tcp/6252/service/api";

/// If it's present, its contents will be used and will have priority over the contents
/// from ./static/controller.id.
///
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
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<8956240>,
    #[b(1)] pub req: T,
    #[n(3)] pub identity_name: Option<String>,
}

impl<T> CloudRequestWrapper<T> {
    pub fn new(req: T, identity_name: Option<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            req,
            identity_name,
        }
    }
}

mod node {
    use std::time::Duration;

    use minicbor::Encode;

    use ockam::identity::{Identifier, SecureChannelOptions, TrustIdentifierPolicy};
    use ockam_core::api::RequestBuilder;
    use ockam_core::compat::str::FromStr;
    use ockam_core::env::get_env;
    use ockam_core::{self, route, Result};
    use ockam_multiaddr::MultiAddr;
    use ockam_node::api::request_with_options;
    use ockam_node::{Context, MessageSendReceiveOptions, DEFAULT_TIMEOUT};

    use crate::cloud::controller_requests::OCKAM_CONTROLLER_IDENTITY_ID;
    use crate::error::ApiError;
    use crate::nodes::{NodeManager, NodeManagerWorker};

    impl NodeManager {
        /// Load controller identity id from file.
        ///
        /// If the env var `OCKAM_CONTROLLER_IDENTITY_ID` is set, that will be used to
        /// load the identifier instead of the file.
        pub fn load_controller_identifier() -> Result<Identifier> {
            if let Ok(Some(idt)) = get_env::<Identifier>(OCKAM_CONTROLLER_IDENTITY_ID) {
                trace!(idt = %idt, "Read controller identifier from env");
                return Ok(idt);
            }
            Identifier::from_str(include_str!("../../static/controller.id"))
        }

        /// Return controller identity's identifier.
        pub fn controller_identifier(&self) -> Identifier {
            self.controller_identity_id.clone()
        }

        #[allow(clippy::too_many_arguments)]
        pub async fn request_controller<T>(
            &self,
            ctx: &Context,
            label: &str,
            schema: impl Into<Option<&str>>,
            api_service: &str,
            req: RequestBuilder<T>,
            ident: Option<String>,
        ) -> Result<Vec<u8>>
        where
            T: Encode<()>,
        {
            self.request_controller_with_timeout(
                ctx,
                label,
                schema,
                api_service,
                req,
                ident,
                Duration::from_secs(DEFAULT_TIMEOUT),
            )
            .await
        }

        #[allow(clippy::too_many_arguments)]
        pub(crate) async fn request_controller_with_timeout<T>(
            &self,
            ctx: &Context,
            label: &str,
            schema: impl Into<Option<&str>>,
            api_service: &str,
            req: RequestBuilder<T>,
            ident: Option<String>,
            timeout: Duration,
        ) -> Result<Vec<u8>>
        where
            T: Encode<()>,
        {
            self.request_node(ctx, None, label, schema, api_service, req, ident, timeout)
                .await
        }

        /// Send a request to a node referenced via its multiaddr
        #[allow(clippy::too_many_arguments)]
        pub(crate) async fn request_node<T>(
            &self,
            ctx: &Context,
            destination: Option<MultiAddr>,
            label: &str,
            schema: impl Into<Option<&str>>,
            api_service: &str,
            req: RequestBuilder<T>,
            ident: Option<String>,
            timeout: Duration,
        ) -> Result<Vec<u8>>
        where
            T: Encode<()>,
        {
            let identity_name = ident.clone();
            let identifier = self.get_identifier(identity_name.clone()).await?;

            let secure_channels = self.secure_channels.clone();
            let cloud_multiaddr = destination.unwrap_or(self.controller_address());
            let sc = {
                let cloud_route = crate::multiaddr_to_route(&cloud_multiaddr, &self.tcp_transport)
                    .await
                    .ok_or_else(|| {
                        ApiError::core(format!(
                    "Couldn't convert MultiAddr to route: cloud_multiaddr={cloud_multiaddr}"
                ))
                    })?;

                let options = SecureChannelOptions::new()
                    .with_trust_policy(TrustIdentifierPolicy::new(self.controller_identifier()));
                secure_channels
                    .create_secure_channel(ctx, &identifier, cloud_route.route, options)
                    .await?
            };

            let route = route![sc.clone(), api_service];
            let options = MessageSendReceiveOptions::new().with_timeout(timeout);
            let res = request_with_options(ctx, label, schema, route, req, options).await;
            secure_channels
                .stop_secure_channel(ctx, sc.encryptor_address())
                .await?;
            res
        }

        pub fn controller_address(&self) -> MultiAddr {
            let address = super::controller_multiaddr();
            trace!(%address, "Controller address");
            address
        }
    }

    impl NodeManagerWorker {
        #[allow(clippy::too_many_arguments)]
        pub(crate) async fn request_controller<T>(
            &self,
            ctx: &Context,
            label: &str,
            schema: impl Into<Option<&str>>,
            api_service: &str,
            req: RequestBuilder<T>,
            ident: Option<String>,
        ) -> Result<Vec<u8>>
        where
            T: Encode<()>,
        {
            self.request_controller_with_timeout(
                ctx,
                label,
                schema,
                api_service,
                req,
                ident,
                Duration::from_secs(DEFAULT_TIMEOUT),
            )
            .await
        }

        #[allow(clippy::too_many_arguments)]
        pub(crate) async fn request_controller_with_timeout<T>(
            &self,
            ctx: &Context,
            label: &str,
            schema: impl Into<Option<&str>>,
            api_service: &str,
            req: RequestBuilder<T>,
            ident: Option<String>,
            timeout: Duration,
        ) -> Result<Vec<u8>>
        where
            T: Encode<()>,
        {
            let node_manager = self.inner().read().await;
            node_manager
                .request_controller_with_timeout(
                    ctx,
                    label,
                    schema,
                    api_service,
                    req,
                    ident,
                    timeout,
                )
                .await
        }
    }
}

pub fn controller_multiaddr() -> MultiAddr {
    let default_addr = MultiAddr::from_string(DEFAULT_CONTROLLER_ADDRESS)
        .unwrap_or_else(|_| panic!("invalid Controller address: {DEFAULT_CONTROLLER_ADDRESS}"));
    get_env_with_default::<MultiAddr>(OCKAM_CONTROLLER_ADDR, default_addr).unwrap()
}
