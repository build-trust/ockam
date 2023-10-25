use crate::cloud::Controller;
use miette::IntoDiagnostic;
use minicbor::{Decode, Encode};
use ockam::identity::Identifier;
use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

#[derive(Encode, Decode, Debug, Default, Clone, Eq, PartialEq)]
pub struct RelayController {
    #[cbor(n(1))]
    pub addr: String,
    #[cbor(n(2))]
    pub tags: Vec<String>,
    #[cbor(n(3))]
    pub target_identifier: Vec<u8>,
    #[cbor(n(4))]
    pub crated_at: u64,
    #[cbor(n(5))]
    pub updated_at: u64,
}

#[async_trait]
pub trait ConntrollerRelays {
    async fn create_relay(
        &self,
        ctx: &Context,
        address: &MultiAddr,
        alias: Option<String>,
        authorized: Option<Identifier>,
    ) -> miette::Result<RelayController>;

    async fn show_relay(
        &self,
        ctx: &Context,
        remote_address: &str,
    ) -> miette::Result<RelayController>;

    async fn list_relays(&self, ctx: &Context) -> miette::Result<Vec<RelayController>>;
}

#[async_trait]
impl ConntrollerRelays for Controller {
    async fn create_relay(
        &self,
        _ctx: &Context,
        _address: &MultiAddr,
        _alias: Option<String>,
        _authorized: Option<Identifier>,
    ) -> miette::Result<RelayController> {
        todo!()
    }

    async fn show_relay(
        &self,
        ctx: &Context,
        remote_address: &str,
    ) -> miette::Result<RelayController> {
        self.0
            .ask(
                ctx,
                "static_forwarding_api",
                Request::get(format!("/{remote_address}")),
            )
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn list_relays(&self, ctx: &Context) -> miette::Result<Vec<RelayController>> {
        self.0
            .ask(ctx, "static_forwarding_api", Request::get("/"))
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}
