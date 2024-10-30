use miette::IntoDiagnostic;
use ockam::identity::Identifier;
use ockam_api::nodes::models::flow_controls::AddConsumer;
use ockam_api::nodes::models::services::{
    StartHopServiceRequest, StartRemoteProxyVaultServiceRequest,
};
use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_api::nodes::*;
use ockam_core::api::Request;
use ockam_core::flow_control::FlowControlId;
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;

use crate::Result;

/// Construct a request to query node status
pub(crate) fn query_status() -> Request<()> {
    Request::get("/node")
}

pub(crate) fn get_node_resources() -> Request<()> {
    Request::get("/node/resources")
}

/// Construct a request to query node tcp listeners
pub(crate) fn list_tcp_listeners() -> Request<()> {
    Request::get("/node/tcp/listener")
}

/// Construct a request to print a list of services for the given node
pub(crate) fn list_services() -> Request<()> {
    Request::get("/node/services")
}

/// Construct a request builder to list all secure channels on the given node
pub(crate) fn list_secure_channels() -> Request<()> {
    Request::get("/node/secure_channel")
}

/// Construct a request builder to list all workers on the given node
pub(crate) fn list_workers() -> Request<()> {
    Request::get("/node/workers")
}

pub(crate) fn delete_secure_channel(
    addr: &Address,
) -> Request<models::secure_channel::DeleteSecureChannelRequest> {
    let payload = models::secure_channel::DeleteSecureChannelRequest::new(addr);
    Request::delete("/node/secure_channel").body(payload)
}

pub(crate) fn show_secure_channel(
    addr: &Address,
) -> Request<models::secure_channel::ShowSecureChannelRequest> {
    let payload = models::secure_channel::ShowSecureChannelRequest::new(addr);
    Request::get("/node/show_secure_channel").body(payload)
}

/// Construct a request to create Secure Channel Listeners
pub(crate) fn create_secure_channel_listener(
    addr: &Address,
    authorized_identifiers: Option<Vec<Identifier>>,
    identity_name: Option<String>,
) -> Result<Vec<u8>> {
    let payload = models::secure_channel::CreateSecureChannelListenerRequest::new(
        addr,
        authorized_identifiers,
        identity_name,
    );

    let mut buf = vec![];
    Request::post("/node/secure_channel_listener")
        .body(payload)
        .encode(&mut buf)
        .into_diagnostic()?;
    Ok(buf)
}

/// Construct a request to list Secure Channel Listeners
pub(crate) fn list_secure_channel_listener() -> Request<()> {
    Request::get("/node/secure_channel_listener")
}

pub(crate) fn delete_secure_channel_listener(
    addr: &Address,
) -> Request<models::secure_channel::DeleteSecureChannelListenerRequest> {
    let payload = models::secure_channel::DeleteSecureChannelListenerRequest::new(addr);
    Request::delete("/node/secure_channel_listener").body(payload)
}

/// Construct a request to show Secure Channel Listener
pub(crate) fn show_secure_channel_listener(
    addr: &Address,
) -> Request<models::secure_channel::ShowSecureChannelListenerRequest> {
    let payload = models::secure_channel::ShowSecureChannelListenerRequest::new(addr);
    Request::get("/node/show_secure_channel_listener").body(payload)
}

/// Construct a request to start a Hop Service
pub(crate) fn start_hop_service(addr: &str) -> Request<StartHopServiceRequest> {
    let payload = StartHopServiceRequest::new(addr);
    Request::post(node_service(DefaultAddress::HOP_SERVICE)).body(payload)
}

pub(crate) fn start_remote_proxy_vault_service(
    addr: &str,
    vault_name: &str,
) -> Request<StartRemoteProxyVaultServiceRequest> {
    let payload = StartRemoteProxyVaultServiceRequest::new(addr, vault_name);
    Request::post(node_service(DefaultAddress::REMOTE_PROXY_VAULT)).body(payload)
}

pub(crate) fn add_consumer(id: FlowControlId, address: MultiAddr) -> Request<AddConsumer> {
    let payload = AddConsumer::new(id, address);
    Request::post("/node/flow_controls/add_consumer").body(payload)
}

/// Return the path of a service given its name
fn node_service(service_name: &str) -> String {
    format!("/node/services/{service_name}")
}
