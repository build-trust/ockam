use super::common::default_authority;
use crate::control_api::protocol::common::Authority;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct AddOrUpdateAuthorityMemberRequest {
    /// Attributes in `key=value` format to be attached to the member;
    #[schema(examples(
        "ockam-role=member",
        "ockam-role=enroller",
        "ockam-relay=foo",
        "my-attribute=my-value"
    ))]
    pub attributes: BTreeMap<String, String>,
    /// Identity to use when contacting the Authority node;
    /// When omitted, the default identity will be used
    #[schema(examples("Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"))]
    pub identity: Option<String>,
    /// Authority to use when creating the ticket;
    #[serde(default = "default_authority")]
    #[schema(default = default_authority)]
    pub authority: Authority,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ListAuthorityMembersRequest {
    /// Identity to use when contacting the Authority node;
    /// When omitted, the default identity will be used
    #[schema(examples("Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"))]
    pub identity: Option<String>,
    /// Authority to use when creating the ticket;
    #[serde(default = "default_authority")]
    #[schema(default = default_authority)]
    pub authority: Authority,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct GetAuthorityMemberRequest {
    /// Identity to use when contacting the Authority node;
    /// When omitted, the default identity will be used
    #[schema(examples("Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"))]
    pub identity: Option<String>,
    /// Authority to use when creating the ticket;
    #[serde(default = "default_authority")]
    #[schema(default = default_authority)]
    pub authority: Authority,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RemoveAuthorityMemberRequest {
    /// Identity to use when contacting the Authority node;
    /// When omitted, the default identity will be used
    #[schema(examples("Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"))]
    pub identity: Option<String>,
    /// Authority to use when creating the ticket;
    #[serde(default = "default_authority")]
    #[schema(default = default_authority)]
    pub authority: Authority,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct AuthorityMember {
    /// Member identity
    #[schema(examples("Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"))]
    pub identity: String,
    /// Attributes attached to the member
    #[schema(examples("ockam-role=member, ockam-relay=foo"))]
    pub attributes: BTreeMap<String, String>,
}
