use crate::error::ApiError;
use core::str;
use minicbor::Decoder;
use ockam::identity::credential::Timestamp;
use ockam_core::api;
use ockam_core::api::{Method, Request, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::{self, Result, Routed, Worker};
use ockam_identity::authenticated_storage::{AttributesEntry, IdentityAttributeStorageWriter};
use ockam_identity::{IdentityIdentifier, IdentitySecureChannelLocalInfo};
use ockam_node::Context;
use reqwest::StatusCode;
use std::collections::HashMap;
use tracing::trace;

pub struct Server {
    project: Vec<u8>,
    store: Arc<dyn IdentityAttributeStorageWriter>,
    tenant_base_url: String,
    certificate: reqwest::Certificate,
    attributes: Vec<String>,
}

#[ockam_core::worker]
impl Worker for Server {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        if let Ok(i) = IdentitySecureChannelLocalInfo::find_info(m.local_message()) {
            let r = self.on_request(i.their_identity_id(), m.as_body()).await?;
            c.send(m.return_route(), r).await
        } else {
            let mut dec = Decoder::new(m.as_body());
            let req: Request = dec.decode()?;
            let res = api::forbidden(&req, "secure channel required").to_vec()?;
            c.send(m.return_route(), res).await
        }
    }
}

impl Server {
    pub fn new(
        project: Vec<u8>,
        store: Arc<dyn IdentityAttributeStorageWriter>,
        tenant_base_url: &str,
        certificate: &str,
        attributes: &[&str],
    ) -> Result<Self> {
        let certificate = reqwest::Certificate::from_pem(certificate.as_bytes())
            .map_err(|err| ApiError::generic(&err.to_string()))?;
        Ok(Server {
            project,
            store,
            tenant_base_url: tenant_base_url.to_string(),
            certificate,
            attributes: attributes.iter().map(|s| s.to_string()).collect(),
        })
    }

    async fn on_request(&mut self, from: &IdentityIdentifier, data: &[u8]) -> Result<Vec<u8>> {
        let mut dec = Decoder::new(data);
        let req: Request = dec.decode()?;

        trace! {
            target: "ockam_api::okta::server",
            from   = %from,
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }
        let res = match req.method() {
            Some(Method::Post) => match req.path_segments::<2>().as_slice() {
                // Device Flow authentication
                ["v0", "enroll"] => {
                    debug!("Checking token for project {:?}", self.project);
                    // TODO: check token_type
                    // TODO: it's AuthenticateAuth0Token or something else?.  Probably rename.
                    let token: crate::cloud::enroll::auth0::AuthenticateAuth0Token =
                        dec.decode()?;
                    debug!("device code received: {token:#?}");
                    if let Some(attrs) = self.check_token(&token.access_token.0).await? {
                        //TODO in some future, we will want to track that this entry
                        //     was added by the okta addon.
                        //     But for that we would need to give a separate identity to this
                        //     addon, and made it an "enroller" (calling the enroll endpoint)
                        let entry = AttributesEntry::new(
                            attrs
                                .into_iter()
                                .map(|(k, v)| (k, v.as_bytes().to_vec()))
                                .chain(
                                    [(
                                        crate::authenticator::direct::PROJECT_ID.to_owned(),
                                        self.project.clone(),
                                    )]
                                    .into_iter(),
                                )
                                .collect(),
                            Timestamp::now().unwrap(),
                            None,
                            None,
                        );
                        self.store.put_attributes(from, entry).await?;
                        Response::ok(req.id()).to_vec()?
                    } else {
                        api::forbidden(&req, "Forbidden").to_vec()?
                    }
                }
                _ => api::unknown_path(&req).to_vec()?,
            },
            _ => api::invalid_method(&req).to_vec()?,
        };
        Ok(res)
    }

    async fn check_token(&mut self, token: &str) -> Result<Option<HashMap<String, String>>> {
        let client = reqwest::ClientBuilder::new()
            .tls_built_in_root_certs(false)
            .add_root_certificate(self.certificate.clone())
            .build()
            .map_err(|err| ApiError::generic(&err.to_string()))?;
        let res = client
            .get(format!("{}/v1/userinfo", &self.tenant_base_url))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await;
        if let Ok(res) = res {
            match res.status() {
                StatusCode::OK => {
                    //TODO: Must have a configured list of fields to extract from the response,
                    //      and add these to the credentia
                    let doc: HashMap<String, serde_json::Value> = res
                        .json()
                        .await
                        .map_err(|_err| ApiError::generic("Failed to authenticate with Okta"))?;
                    debug!("userinfo received: {doc:?}");
                    let mut custom_attrs = HashMap::new();
                    for a in self.attributes.iter() {
                        if let Some(v) = doc.get(a).and_then(|v| v.as_str()) {
                            custom_attrs.insert(a.to_owned(), v.to_string());
                        }
                    }
                    Ok(Some(custom_attrs))
                }
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
}
