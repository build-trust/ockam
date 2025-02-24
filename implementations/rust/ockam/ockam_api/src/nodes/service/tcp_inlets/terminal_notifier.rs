use crate::colors::color_primary;
use crate::nodes::NodeManager;
use crate::{fmt_info, fmt_ok, fmt_warn};
use colorful::Colorful;
use ockam_multiaddr::MultiAddr;
use std::sync::Weak;

pub(super) struct TcpInletNotifier {
    node_manager: Weak<NodeManager>,
    inlet_name: String,
}

impl TcpInletNotifier {
    pub(super) fn new(node_manager: Weak<NodeManager>, inlet_name: String) -> Self {
        Self {
            node_manager,
            inlet_name,
        }
    }
    pub(super) async fn on_session_down(&self, outlet_address: &MultiAddr) {
        if let Some(node_manager) = self.node_manager.upgrade() {
            if let Some(inlet_handle) = node_manager.registry.inlets.get(&self.inlet_name) {
                let message = fmt_warn!(
                    "The TCP Inlet {} listening at {} lost the connection to the TCP Outlet at {}\n",
                    color_primary(&self.inlet_name),
                    color_primary(inlet_handle.tcp_inlet.socket_address()),
                    color_primary(outlet_address)
                );

                let summary = inlet_handle.summary().await;

                let message = if summary.active_routes.is_empty() {
                    message + &fmt_info!("Every route to TCP Outlet is disconnected.\n",)
                } else {
                    message
                        + &fmt_info!(
                            "There are still {} routes connected to the TCP Outlet\n",
                            summary.active_routes.len()
                        )
                };

                node_manager
                    .cli_state
                    .notify_message(message + &fmt_info!("Attempting to reconnect...\n"));
            }
        }
    }

    pub(super) async fn on_session_replaced(&self, outlet_address: &MultiAddr) {
        if let Some(node_manager) = self.node_manager.upgrade() {
            if let Some(inlet_handle) = node_manager.registry.inlets.get(&self.inlet_name) {
                let message = fmt_ok!(
                    "The TCP Inlet {} listening at {} has restored the connection to the TCP Outlet at {}\n",
                    color_primary(&self.inlet_name),
                    color_primary(inlet_handle.tcp_inlet.socket_address()),
                    color_primary(outlet_address)
                );

                node_manager.cli_state.notify_message(message);
            }
        }
    }
}
