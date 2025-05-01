use minicbor::{CborLen, Decode, Encode};
use ockam::{identity::Identifier, Context, Decodable, Encodable, Encoded, Message};
use ockam_core::api::Request;
use ockam_core::cbor_encode_preallocate;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
};

use crate::{
    colors::color_primary,
    orchestrator::{HasSecureClient, ProjectNodeClient},
    output::Output,
};

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Clone, Message)]
#[cbor(map)]
pub struct ProjectRelay {
    #[cbor(n(1))]
    pub addr: String,

    #[cbor(n(2))]
    pub tags: BTreeMap<String, String>,

    #[cbor(n(3))]
    pub target_identifier: Identifier,

    #[cbor(n(4))]
    pub created_at: i64,

    #[cbor(n(5))]
    pub updated_at: i64,
}

impl Encodable for ProjectRelay {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for ProjectRelay {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl Display for ProjectRelay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", color_primary(&self.addr))?;
        Ok(())
    }
}

impl Output for ProjectRelay {
    fn item(&self) -> crate::Result<String> {
        Ok(self.padded_display())
    }
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Clone, Message)]
#[cbor(transparent)]
pub struct ProjectRelayList(#[n(0)] pub Vec<ProjectRelay>);

impl Encodable for ProjectRelayList {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for ProjectRelayList {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(ProjectRelayList(minicbor::decode(e)?))
    }
}

impl ProjectNodeClient {
    pub async fn list_relays(&self, ctx: &Context) -> crate::Result<Vec<ProjectRelay>> {
        trace!("listing project relays");
        let req = Request::get("/");
        let res: ProjectRelayList = self
            .get_secure_client()
            .ask(ctx, "static_forwarding_api", req)
            .await?
            .miette_success("project relay list")?;
        Ok(res.0)
    }
}
