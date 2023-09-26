use minicbor::Decoder;
use ockam::identity::utils::now;
use ockam::identity::{secure_channel_required, TRUST_CONTEXT_ID};
use ockam::identity::{AttributesEntry, IdentityAttributesReader, IdentityAttributesWriter};
use ockam::identity::{Identifier, IdentitySecureChannelLocalInfo};
use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::{CowStr, Result, Routed, Worker};
use ockam_node::Context;
use std::collections::HashMap;
use tracing::trace;

use crate::authenticator::direct::types::AddMember;

pub struct DirectAuthenticator {
    trust_context: String,
    attributes_writer: Arc<dyn IdentityAttributesWriter>,
    attributes_reader: Arc<dyn IdentityAttributesReader>,
}

impl DirectAuthenticator {
    pub async fn new(
        trust_context: String,
        attributes_writer: Arc<dyn IdentityAttributesWriter>,
        attributes_reader: Arc<dyn IdentityAttributesReader>,
    ) -> Result<Self> {
        Ok(Self {
            trust_context,
            attributes_writer,
            attributes_reader,
        })
    }

    async fn add_member<'a>(
        &self,
        enroller: &Identifier,
        id: &Identifier,
        attrs: &HashMap<CowStr<'a>, CowStr<'a>>,
    ) -> Result<()> {
        let auth_attrs = attrs
            .iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
            .chain(
                [(
                    TRUST_CONTEXT_ID.to_owned(),
                    self.trust_context.as_bytes().to_vec(),
                )]
                .into_iter(),
            )
            .collect();
        let entry = AttributesEntry::new(auth_attrs, now()?, None, Some(enroller.clone()));
        self.attributes_writer.put_attributes(id, entry).await
    }

    async fn list_members(&self) -> Result<HashMap<Identifier, AttributesEntry>> {
        let all_attributes = self.attributes_reader.list().await?;
        let attested_by_me = all_attributes.into_iter().collect();
        Ok(attested_by_me)
    }
}

#[ockam_core::worker]
impl Worker for DirectAuthenticator {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        if let Ok(i) = IdentitySecureChannelLocalInfo::find_info(m.local_message()) {
            let from = i.their_identity_id();
            let mut dec = Decoder::new(m.as_body());
            let req: RequestHeader = dec.decode()?;
            trace! {
                target: "ockam_api::authenticator::direct::direct_authenticator",
                from   = %from,
                id     = %req.id(),
                method = ?req.method(),
                path   = %req.path(),
                body   = %req.has_body(),
                "request"
            }
            let path_segments = req.path_segments::<5>();
            let res = match (req.method(), path_segments.as_slice()) {
                (Some(Method::Post), [""]) | (Some(Method::Post), ["members"]) => {
                    let add: AddMember = dec.decode()?;
                    self.add_member(&from, add.member(), add.attributes())
                        .await?;
                    Response::ok(&req).to_vec()?
                }
                (Some(Method::Get), ["member_ids"]) => {
                    let entries = self.list_members().await?;
                    let ids: Vec<Identifier> = entries.into_keys().collect();
                    Response::ok(&req).body(ids).to_vec()?
                }
                (Some(Method::Get), [""]) | (Some(Method::Get), ["members"]) => {
                    let entries = self.list_members().await?;

                    Response::ok(&req).body(entries).to_vec()?
                }
                (Some(Method::Delete), [id]) | (Some(Method::Delete), ["members", id]) => {
                    let identifier = Identifier::try_from(id.to_string())?;
                    self.attributes_writer.delete(&identifier).await?;

                    Response::ok(&req).to_vec()?
                }

                _ => Response::unknown_path(&req).to_vec()?,
            };
            c.send(m.return_route(), res).await
        } else {
            secure_channel_required(c, m).await
        }
    }
}
