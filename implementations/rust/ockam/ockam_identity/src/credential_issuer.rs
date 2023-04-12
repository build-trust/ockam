use crate::alloc::string::ToString;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, AsyncTryClone, Error, Message, Result, Route, Routed, Worker};

use ockam_node::{Context, MessageSendReceiveOptions};
use ockam_vault::Vault;
use CredentialIssuerRequest::*;
use CredentialIssuerResponse::*;

use crate::authenticated_storage::{
    AttributesEntry, AuthenticatedAttributeStorage, IdentityAttributeStorageReader,
    IdentityAttributeStorageWriter,
};
use crate::credential::{Credential, Timestamp};
use crate::{Identity, IdentityIdentifier, PublicIdentity};
use ockam_core::flow_control::FlowControls;
use serde::{Deserialize, Serialize};

/// This struct provides a simplified credential issuer which can be used in test scenarios
/// by starting it as a Worker on any given node.
///
/// note: the storage associated to the issuer identity will not persist between runs.
///
/// You can store attributes for a given identifier using the `put_attribute` function
/// ```no_compile
///     let issuer = CredentialIssuer::create(&ctx).await?;
///     let alice = "P529d43ac7b01e23d3818d00e083508790bfe8825714644b98134db6c1a7a6602".try_into()?;
///     issuer.put_attribute_value(&alice, "name", "alice").await?;
///```
///
/// A Credential for a given identity can then be retrieved with the `get_credential` method.
///
pub struct CredentialIssuer {
    identity: Identity,
    flow_controls: FlowControls,
}

impl CredentialIssuer {
    /// Create a fully in-memory issuer for testing
    pub async fn create(ctx: &Context, flow_controls: &FlowControls) -> Result<CredentialIssuer> {
        let identity = Identity::create(ctx, Vault::create()).await?;
        Ok(CredentialIssuer {
            identity,
            flow_controls: flow_controls.clone(),
        })
    }

    /// Create a new CredentialIssuer from an Identity
    pub fn new(identity: Identity, flow_controls: &FlowControls) -> CredentialIssuer {
        CredentialIssuer {
            identity,
            flow_controls: flow_controls.clone(),
        }
    }

    /// Return the identity holding credentials
    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    /// Return the attributes storage for the issuer identity
    async fn attributes_storage(&self) -> Result<AuthenticatedAttributeStorage> {
        Ok(AuthenticatedAttributeStorage::new(
            self.identity.authenticated_storage().clone(),
        ))
    }

    /// Store an attribute name/value pair for a given identity
    pub async fn put_attribute_value(
        &self,
        subject: &IdentityIdentifier,
        attribute_name: &str,
        attribute_value: &str,
    ) -> Result<()> {
        let attributes_storage: AuthenticatedAttributeStorage = self.attributes_storage().await?;
        let mut attributes = match attributes_storage.get_attributes(subject).await? {
            Some(entry) => (*entry.attrs()).clone(),
            None => BTreeMap::new(),
        };
        attributes.insert(
            attribute_name.to_string(),
            attribute_value.as_bytes().to_vec(),
        );
        let entry = AttributesEntry::new(
            attributes,
            Timestamp::now().unwrap(),
            None,
            Some(self.identity.identifier().clone()),
        );
        attributes_storage.put_attributes(subject, entry).await
    }
}

#[ockam_core::async_trait]
impl CredentialIssuerApi for CredentialIssuer {
    /// Return the issuer public identity
    async fn public_identity(&self, _options: MessageSendReceiveOptions) -> Result<PublicIdentity> {
        self.identity.to_public().await
    }

    /// Create an authenticated credential for an identity
    async fn get_credential(
        &self,
        subject: &IdentityIdentifier,
        _options: MessageSendReceiveOptions,
    ) -> Result<Option<Credential>> {
        let mut builder = Credential::builder(subject.clone());
        let identity_attributes: AuthenticatedAttributeStorage = self.attributes_storage().await?;
        if let Some(attributes) = identity_attributes.get_attributes(subject).await? {
            builder =
                attributes
                    .attrs()
                    .iter()
                    .fold(builder, |b, (attribute_name, attribute_value)| {
                        b.with_attribute(attribute_name, attribute_value.as_slice())
                    });
            let credential = self.identity.issue_credential(builder).await?;
            Ok(Some(credential))
        } else {
            Ok(None)
        }
    }
}

/// This trait provides an interface for a CredentialIssuer so that it can be called directly
/// or via a worker by sending messages
#[ockam_core::async_trait]
pub trait CredentialIssuerApi {
    /// Return the issuer public identity
    async fn public_identity(&self, options: MessageSendReceiveOptions) -> Result<PublicIdentity>;

    /// Return an authenticated credential a given identity
    async fn get_credential(
        &self,
        subject: &IdentityIdentifier,
        options: MessageSendReceiveOptions,
    ) -> Result<Option<Credential>>;
}

/// Worker implementation for a CredentialIssuer
/// This worker provides an API to the CredentialIssuer in order to:
///   - get a credential
///   - get the issuer public identity in order to verify credentials locally
#[ockam_core::worker]
impl Worker for CredentialIssuer {
    type Message = CredentialIssuerRequest;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<CredentialIssuerRequest>,
    ) -> Result<()> {
        let return_route = msg.return_route();
        match msg.body() {
            GetCredential(subject) => {
                let credential = self
                    .get_credential(
                        &subject,
                        MessageSendReceiveOptions::new().with_flow_control(&self.flow_controls),
                    )
                    .await?;
                ctx.send(return_route, CredentialResponse(credential)).await
            }
            GetPublicIdentity => {
                let identity = self
                    .public_identity(
                        MessageSendReceiveOptions::new().with_flow_control(&self.flow_controls),
                    )
                    .await?;
                ctx.send(return_route, PublicIdentityResponse(identity))
                    .await
            }
        }
    }
}

/// Requests for the CredentialIssuer worker API
#[derive(ockam_core::Message, Serialize, Deserialize)]
pub enum CredentialIssuerRequest {
    /// get an authenticated credential for a given identity
    GetCredential(IdentityIdentifier),
    /// get the public identity of the issuer
    GetPublicIdentity,
}

/// Responses for the CredentialIssuer worker API
#[derive(ockam_core::Message, Serialize, Deserialize)]
pub enum CredentialIssuerResponse {
    /// return an authenticated credential
    CredentialResponse(Option<Credential>),
    /// return the public identity of the issuer
    PublicIdentityResponse(PublicIdentity),
}

/// Client access to an CredentialIssuer worker
pub struct CredentialIssuerClient {
    ctx: Context,
    credential_issuer_route: Route,
}

impl CredentialIssuerClient {
    /// Create an access to an CredentialIssuer worker given a route to that worker
    /// It uses a Context to send and receive messages
    pub async fn new(ctx: &Context, issuer_route: Route) -> Result<CredentialIssuerClient> {
        Ok(CredentialIssuerClient {
            ctx: ctx.async_try_clone().await?,
            credential_issuer_route: issuer_route,
        })
    }
}

#[ockam_core::async_trait]
impl CredentialIssuerApi for CredentialIssuerClient {
    async fn public_identity(&self, options: MessageSendReceiveOptions) -> Result<PublicIdentity> {
        let response = self
            .ctx
            .send_and_receive_extended::<CredentialIssuerResponse>(
                route![self.credential_issuer_route.clone(), "issuer"],
                GetPublicIdentity,
                options,
            )
            .await?
            .body();
        match response {
            PublicIdentityResponse(identity) => Ok(identity),
            _ => Err(error("missing public identity for the credential issuer")),
        }
    }

    async fn get_credential(
        &self,
        subject: &IdentityIdentifier,
        options: MessageSendReceiveOptions,
    ) -> Result<Option<Credential>> {
        let response = self
            .ctx
            .send_and_receive_extended::<CredentialIssuerResponse>(
                route![self.credential_issuer_route.clone(), "issuer"],
                GetCredential(subject.clone()),
                options,
            )
            .await?
            .body();
        match response {
            CredentialResponse(credential) => Ok(credential),
            _ => Err(error("missing credential")),
        }
    }
}

fn error(message: &str) -> Error {
    Error::new(Origin::Application, Kind::Invalid, message.to_string())
}
