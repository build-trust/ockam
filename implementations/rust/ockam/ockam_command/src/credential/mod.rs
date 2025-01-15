use clap::{Args, Subcommand};
use colorful::Colorful;
use miette::miette;
use serde::Serialize;
use serde_json::json;

pub(crate) use issue::IssueCommand;
use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::{Identifier, TimestampInSeconds};
use ockam_api::output::Output;
use ockam_api::terminal::fmt;
use ockam_core::compat::collections::HashMap;
use ockam_node::Context;
pub(crate) use store::StoreCommand;
pub(crate) use verify::VerifyCommand;

use crate::credential::list::ListCommand;
use crate::{CommandGlobalOpts, Result};

pub(crate) mod issue;
pub(crate) mod list;
pub(crate) mod store;
pub(crate) mod verify;

/// Manage Credentials
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct CredentialCommand {
    #[command(subcommand)]
    subcommand: CredentialSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CredentialSubcommand {
    List(ListCommand),
    Issue(IssueCommand),
    Store(StoreCommand),
    Verify(VerifyCommand),
}

impl CredentialSubcommand {
    pub fn name(&self) -> String {
        match &self {
            CredentialSubcommand::List(c) => c.name(),
            CredentialSubcommand::Issue(c) => c.name(),
            CredentialSubcommand::Store(c) => c.name(),
            CredentialSubcommand::Verify(c) => c.name(),
        }
    }
}

impl CredentialCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            CredentialSubcommand::List(c) => c.run(opts).await,
            CredentialSubcommand::Issue(c) => c.run(opts).await,
            CredentialSubcommand::Store(c) => c.run(ctx, opts).await,
            CredentialSubcommand::Verify(c) => c.run(opts).await,
        }
    }

    pub fn name(&self) -> String {
        self.subcommand.name()
    }
}

#[derive(Serialize)]
pub struct CredentialOutput {
    pub credential: String,
    pub subject: Identifier,
    pub issuer: Identifier,
    pub created_at: TimestampInSeconds,
    pub expires_at: TimestampInSeconds,
    pub attributes: HashMap<String, String>,
}

impl CredentialOutput {
    pub fn from_credential(credential: CredentialAndPurposeKey) -> Result<Self> {
        let str = hex::encode(credential.encode_as_cbor_bytes()?);
        let credential_data = credential.credential.get_credential_data()?;
        let purpose_key_data = credential.purpose_key_attestation.get_attestation_data()?;

        let subject = credential_data
            .subject
            .ok_or(miette!("credential subject is missing"))?;

        let mut attributes = HashMap::<String, String>::default();
        for (k, v) in credential_data.subject_attributes.map {
            let k = String::from_utf8(k.to_vec()).unwrap_or("**binary**".to_string());
            let v = String::from_utf8(v.to_vec()).unwrap_or("**binary**".to_string());
            attributes.insert(k, v);
        }

        Ok(Self {
            credential: str,
            subject,
            issuer: purpose_key_data.subject,
            created_at: credential_data.created_at,
            expires_at: credential_data.expires_at,
            attributes,
        })
    }
}

impl Output for CredentialOutput {
    fn item(&self) -> ockam_api::Result<String> {
        let attributes = json!(self.attributes).to_string();

        let output = format!(
            "{pad}Credential:\n\
             {pad}{ind}subject: {subject}\n\
             {pad}{ind}issuer: {issuer}\n\
             {pad}{ind}created at: {created_at}\n\
             {pad}{ind}expires at: {expires_at}\n\
             {pad}{ind}attributes: {attributes}\n\
             {pad}{ind}:credential {credential}",
            pad = fmt::PADDING,
            ind = fmt::INDENTATION,
            subject = self.subject,
            issuer = self.issuer,
            created_at = self.created_at.0,
            expires_at = self.expires_at.0,
            attributes = attributes,
            credential = self.credential,
        );

        Ok(output)
    }

    fn as_list_item(&self) -> ockam_api::Result<String> {
        let attributes = json!(self.attributes).to_string();

        let output = format!(
            "Credential:\n\
             {ind}subject: {subject}\n\
             {ind}issuer: {issuer}\n\
             {ind}created at: {created_at}\n\
             {ind}expires at: {expires_at}\n\
             {ind}attributes: {attributes}\n\
             {ind}credential: {credential}",
            ind = fmt::INDENTATION,
            subject = self.subject,
            issuer = self.issuer,
            created_at = self.created_at.0,
            expires_at = self.expires_at.0,
            attributes = attributes,
            credential = self.credential,
        );

        Ok(output)
    }
}

#[derive(Serialize)]
pub struct LocalCredentialOutput {
    credential: CredentialOutput,
    scope: String,
    is_verified: bool,
}

impl LocalCredentialOutput {
    pub fn from_credential(
        credential: CredentialAndPurposeKey,
        scope: String,
        is_verified: bool,
    ) -> Result<Self> {
        let s = Self {
            credential: CredentialOutput::from_credential(credential)?,
            scope,
            is_verified,
        };

        Ok(s)
    }
}

impl Output for LocalCredentialOutput {
    fn item(&self) -> ockam_api::Result<String> {
        let is_verified = if self.is_verified {
            "✔︎".light_green()
        } else {
            "✕".light_red()
        };

        let output = format!(
            "{output}\n\
             {pad}{ind}is verified: {is_verified}\n\
             {pad}{ind}scope: {scope}",
            output = self.credential.item()?,
            pad = fmt::PADDING,
            ind = fmt::INDENTATION,
            is_verified = is_verified,
            scope = self.scope,
        );

        Ok(output)
    }

    fn as_list_item(&self) -> ockam_api::Result<String> {
        let is_verified = if self.is_verified {
            "✔︎".light_green()
        } else {
            "✕".light_red()
        };

        let output = format!(
            "{output}\n\
             {ind}is verified: {is_verified}\n\
             {ind}scope: {scope}",
            output = self.credential.as_list_item()?,
            ind = fmt::INDENTATION,
            is_verified = is_verified,
            scope = self.scope,
        );

        Ok(output)
    }
}
