use super::common::{default_project_information, Attributes, HostPortResponse, Project};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

fn default_usage_count() -> u64 {
    1
}

fn default_expires_in() -> u64 {
    600
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CreateTicketRequest {
    pub attributes: Attributes,

    /// Identity to use when contacting the Authority node.
    /// When omitted, the default identity will be used.
    #[schema(examples("Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"))]
    pub identity: Option<String>,

    /// Project information to use when creating the ticket.
    #[serde(default = "default_project_information")]
    #[schema(default = default_project_information)]
    pub project: Project,

    /// Number of times the ticket can be used to enroll.
    #[serde(default = "default_usage_count")]
    #[schema(default = default_usage_count)]
    pub usage_count: u64,

    /// Duration for which the enrollment ticket is valid.
    /// The value is expressed in seconds.
    #[serde(default = "default_expires_in")]
    #[schema(default = default_expires_in)]
    pub expires_in: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Ticket {
    /// Encoded ticket.
    /// The ticket is encoded in a format understood by the Ockam Command and it may vary over time.
    pub encoded: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnrollProjectRequest {
    /// Identity to enroll.
    /// When omitted, the node's identity will be used.
    #[schema(examples("Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"))]
    pub identity: Option<String>,

    /// Ticket to use for enrollment.
    pub ticket: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthorityInformation {
    /// Route to the authority node.
    pub route: String,

    /// Identity of the authority node.
    pub identity: String,

    /// Hostname and port of the authority node.
    pub address: HostPortResponse,
}
