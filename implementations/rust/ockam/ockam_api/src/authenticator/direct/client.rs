use core::str;
use ockam::identity::AttributesEntry;
use ockam::identity::Identifier;
use ockam_core::api::Request;
use ockam_core::Result;
use ockam_node::RpcClient;
use std::collections::HashMap;

use crate::authenticator::direct::types::AddMember;

pub struct DirectAuthenticatorClient(RpcClient);

impl DirectAuthenticatorClient {
    pub fn new(client: RpcClient) -> Self {
        DirectAuthenticatorClient(client)
    }

    pub async fn add_member(&self, id: Identifier, attributes: HashMap<&str, &str>) -> Result<()> {
        self.0
            .request_no_resp_body(
                &Request::post("/").body(AddMember::new(id).with_attributes(attributes)),
            )
            .await
    }

    pub async fn list_member_ids(&self) -> Result<Vec<Identifier>> {
        self.0.request(&Request::get("/member_ids")).await
    }

    pub async fn list_members(&self) -> Result<HashMap<Identifier, AttributesEntry>> {
        self.0.request(&Request::get("/")).await
    }

    pub async fn delete_member(&self, id: Identifier) -> Result<()> {
        self.0
            .request_no_resp_body(&Request::delete(format!("/{id}")))
            .await
    }
}
