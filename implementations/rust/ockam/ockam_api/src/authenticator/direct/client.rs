use miette::IntoDiagnostic;
use std::collections::{BTreeMap, HashMap};

use ockam::identity::models::IdentifierList;
use ockam::identity::AttributesEntry;
use ockam::identity::Identifier;
use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;

use crate::authenticator::direct::types::{AddMember, MemberList};
use crate::nodes::service::default_address::DefaultAddress;
use crate::orchestrator::{AuthorityNodeClient, HasSecureClient};

#[async_trait]
pub trait Members {
    async fn add_member(
        &self,
        ctx: &Context,
        identifier: Identifier,
        attributes: BTreeMap<String, String>,
    ) -> miette::Result<()>;

    async fn show_member(
        &self,
        ctx: &Context,
        identifier: &Identifier,
    ) -> miette::Result<AttributesEntry>;

    async fn delete_all_members(&self, ctx: &Context) -> miette::Result<()>;
    async fn delete_member(&self, ctx: &Context, identifier: &Identifier) -> miette::Result<()>;

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

    async fn show_member(
        &self,
        ctx: &Context,
        identifier: &Identifier,
    ) -> miette::Result<AttributesEntry> {
        let req = Request::get(format!("/{identifier}"));
        self.get_secure_client()
            .ask(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn delete_all_members(&self, ctx: &Context) -> miette::Result<()> {
        let req = Request::delete("/members");
        self.get_secure_client()
            .tell(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn delete_member(&self, ctx: &Context, identifier: &Identifier) -> miette::Result<()> {
        let req = Request::delete(format!("/{identifier}"));
        self.get_secure_client()
            .tell(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn list_member_ids(&self, ctx: &Context) -> miette::Result<Vec<Identifier>> {
        let req = Request::get("/member_ids");
        let identifiers: IdentifierList = self
            .get_secure_client()
            .ask(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()?;
        Ok(identifiers.0)
    }

    async fn list_members(
        &self,
        ctx: &Context,
    ) -> miette::Result<HashMap<Identifier, AttributesEntry>> {
        let req = Request::get("/");
        let member_list: MemberList = self
            .get_secure_client()
            .ask(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()?;
        Ok(member_list.0)
    }
}
