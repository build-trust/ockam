use miette::IntoDiagnostic;
use std::collections::{BTreeMap, HashMap};

use ockam::identity::AttributesEntry;
use ockam::identity::Identifier;
use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;

use crate::authenticator::direct::types::AddMember;
use crate::cloud::{AuthorityNodeClient, HasSecureClient};
use crate::nodes::service::default_address::DefaultAddress;

#[async_trait]
pub trait Members {
    async fn add_member(
        &self,
        ctx: &Context,
        identifier: Identifier,
        attributes: BTreeMap<String, String>,
    ) -> miette::Result<()>;

    async fn delete_member(&self, ctx: &Context, identifier: Identifier) -> miette::Result<()>;

    async fn delete_all_members(&self, ctx: &Context, except: Identifier) -> miette::Result<()>;

    async fn list_member_ids(&self, ctx: &Context) -> miette::Result<Vec<Identifier>>;

    async fn list_members(
        &self,
        ctx: &Context,
    ) -> miette::Result<HashMap<Identifier, AttributesEntry>>;
}

#[async_trait]
impl Members for AuthorityNodeClient {
    async fn add_member(
        &self,
        ctx: &Context,
        identifier: Identifier,
        attributes: BTreeMap<String, String>,
    ) -> miette::Result<()> {
        let req = Request::post("/").body(AddMember::new(identifier).with_attributes(attributes));
        self.get_secure_client()
            .tell(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn delete_member(&self, ctx: &Context, identifier: Identifier) -> miette::Result<()> {
        let req = Request::delete(format!("/{identifier}"));
        self.get_secure_client()
            .tell(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn delete_all_members(&self, ctx: &Context, except: Identifier) -> miette::Result<()> {
        let member_ids = self.list_member_ids(ctx).await?;
        for id in member_ids {
            if id != except {
                if let Err(e) = self.delete_member(ctx, id.clone()).await {
                    warn!("Failed to delete member {}: {}", id, e);
                }
            }
        }
        Ok(())
    }

    async fn list_member_ids(&self, ctx: &Context) -> miette::Result<Vec<Identifier>> {
        let req = Request::get("/member_ids");
        self.get_secure_client()
            .ask(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn list_members(
        &self,
        ctx: &Context,
    ) -> miette::Result<HashMap<Identifier, AttributesEntry>> {
        let req = Request::get("/");
        self.get_secure_client()
            .ask(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}
