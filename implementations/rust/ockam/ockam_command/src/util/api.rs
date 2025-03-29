use ockam_api::nodes::models::flow_controls::AddConsumer;
use ockam_api::nodes::models::services::StartHopServiceRequest;
use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_api::nodes::*;
use ockam_core::api::Request;
use ockam_core::flow_control::FlowControlId;
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;

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

pub(crate) fn add_consumer(id: FlowControlId, address: MultiAddr) -> Request<AddConsumer> {
    let payload = AddConsumer::new(id, address);
    Request::post("/node/flow_controls/add_consumer").body(payload)
}

/// Return the path of a service given its name
fn node_service(service_name: &str) -> String {
    format!("/node/services/{service_name}")
}
