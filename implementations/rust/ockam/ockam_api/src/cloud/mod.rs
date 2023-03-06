use std::str::FromStr;

use minicbor::{Decode, Encode};

use crate::error::ApiError;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{CowStr, Result};
use ockam_multiaddr::MultiAddr;

pub mod addon;
pub mod enroll;
pub mod lease_manager;
pub mod project;
pub mod space;
pub mod subscription;

/// If it's present, its contents will be used and will have priority over the contents
/// from ./static/controller.id.
///
/// How to use: when running a command that spawns a background node or use an embedded node
/// add the env variable. `OCKAM_CONTROLLER_IDENTITY_ID={identity.id-contents} ockam ...`
pub(crate) const OCKAM_CONTROLLER_IDENTITY_ID: &str = "OCKAM_CONTROLLER_IDENTITY_ID";

/// A default timeout in seconds
pub const ORCHESTRATOR_RESTART_TIMEOUT: u64 = 180;

pub type ProjectAddress = CowStr<'static>;

/// A wrapper around a cloud request with extra fields.
#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CloudRequestWrapper<'a, T> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<8956240>,
    #[b(1)] pub req: T,
    #[b(2)] route: CowStr<'a>,
    #[b(3)] pub identity_name: Option<CowStr<'a>>,
}

impl<'a, T> CloudRequestWrapper<'a, T> {
    pub fn new<S: Into<CowStr<'a>>>(req: T, route: &MultiAddr, identity_name: Option<S>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            req,
            route: route.to_string().into(),
            identity_name: identity_name.map(|x| x.into()),
        }
    }

    pub fn multiaddr(&self) -> Result<MultiAddr> {
        MultiAddr::from_str(self.route.as_ref())
            .map_err(|_err| ApiError::generic(&format!("Invalid route: {}", self.route)))
    }
}

/// A CloudRequestWrapper without an internal request.
pub type BareCloudRequestWrapper<'a> = CloudRequestWrapper<'a, ()>;

impl<'a> BareCloudRequestWrapper<'a> {
    pub fn bare(route: &MultiAddr) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            req: (),
            route: route.to_string().into(),
            identity_name: None,
        }
    }
}

mod node {
    use std::env;
    use std::str::FromStr;
    use std::time::Duration;

    use minicbor::Encode;
    use rust_embed::EmbeddedFile;

    use ockam_core::api::RequestBuilder;
    use ockam_core::{self, route, AsyncTryClone, CowStr, Result};
    use ockam_identity::{IdentityIdentifier, SecureChannelTrustOptions, TrustIdentifierPolicy};
    use ockam_multiaddr::MultiAddr;
    use ockam_node::api::request_with_timeout;
    use ockam_node::{Context, DEFAULT_TIMEOUT};

    use crate::cloud::OCKAM_CONTROLLER_IDENTITY_ID;
    use crate::error::ApiError;
    use crate::nodes::{NodeManager, NodeManagerWorker};
    use crate::StaticFiles;

    impl NodeManager {
        /// Load controller identity id from file.
        ///
        /// If the env var `OCKAM_CONTROLLER_IDENTITY_ID` is set, that will be used to
        /// load the identity instead of the file.
        pub(crate) fn load_controller_identity_id() -> Result<IdentityIdentifier> {
            if let Ok(s) = env::var(OCKAM_CONTROLLER_IDENTITY_ID) {
                trace!(idt = %s, "Read controller identity id from env");
                return IdentityIdentifier::from_str(&s);
            }
            match StaticFiles::get("controller.id") {
                Some(EmbeddedFile { data, .. }) => {
                    let s = core::str::from_utf8(data.as_ref()).map_err(|err| {
                        ApiError::generic(&format!("Failed to parse controller identity id: {err}"))
                    })?;
                    trace!(idt = %s, "Read controller identity id from file");
                    IdentityIdentifier::from_str(s)
                }
                None => Err(ApiError::generic(
                    "Failed to import controller identity id from file",
                )),
            }
        }

        /// Return controller identity's identifier.
        pub(crate) fn controller_identity_id(&self) -> IdentityIdentifier {
            self.controller_identity_id.clone()
        }
    }

    impl NodeManagerWorker {
        #[allow(clippy::too_many_arguments)]
        pub(super) async fn request_controller<T>(
            &mut self,
            ctx: &Context,
            label: &str,
            schema: impl Into<Option<&str>>,
            cloud_multiaddr: &MultiAddr,
            api_service: &str,
            req: RequestBuilder<'_, T>,
            ident: Option<CowStr<'_>>,
        ) -> Result<Vec<u8>>
        where
            T: Encode<()>,
        {
            self.request_controller_with_timeout(
                ctx,
                label,
                schema,
                cloud_multiaddr,
                api_service,
                req,
                ident,
                Duration::from_secs(DEFAULT_TIMEOUT),
            )
            .await
        }

        #[allow(clippy::too_many_arguments)]
        pub(super) async fn request_controller_with_timeout<T>(
            &mut self,
            ctx: &Context,
            label: &str,
            schema: impl Into<Option<&str>>,
            cloud_multiaddr: &MultiAddr,
            api_service: &str,
            req: RequestBuilder<'_, T>,
            ident: Option<CowStr<'_>>,
            timeout: Duration,
        ) -> Result<Vec<u8>>
        where
            T: Encode<()>,
        {
            let identity = {
                let node_manager = self.get().read().await;
                match &ident {
                    Some(existing_identity_name) => {
                        let identity_state = node_manager
                            .cli_state
                            .identities
                            .get(existing_identity_name.as_ref())?;
                        match identity_state.get(ctx, node_manager.vault()?).await {
                            Ok(idt) => idt,
                            Err(_) => {
                                let vault_state = node_manager.cli_state.vaults.default()?;
                                identity_state.get(ctx, &vault_state.get().await?).await?
                            }
                        }
                    }
                    None => node_manager.identity()?.async_try_clone().await?,
                }
            };
            let sc = {
                let node_manager = self.get().read().await;
                let cloud_session =
                    crate::create_tcp_session(cloud_multiaddr, &node_manager.tcp_transport)
                        .await
                        .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;
                let trust_options = SecureChannelTrustOptions::new().with_trust_policy(
                    TrustIdentifierPolicy::new(node_manager.controller_identity_id()),
                );
                let trust_options = if let Some((sessions, session_id)) = cloud_session.session {
                    trust_options.with_ciphertext_session(&sessions, &session_id)
                } else {
                    trust_options
                };
                identity
                    .create_secure_channel(cloud_session.route, trust_options)
                    .await?
            };
            let route = route![&sc.to_string(), api_service];
            let res = request_with_timeout(ctx, label, schema, route, req, timeout).await;
            ctx.stop_worker(sc).await?;
            res
        }
    }
}
