pub mod addresses;
mod inlet_listener;
mod inlet_shared_state;
mod inlet_sni_root_listener;
mod interceptor;
pub mod options;
mod outlet_listener;
mod outlet_listener_registry;
mod portal_message;
mod portal_receiver;
mod portal_worker;
mod tls_certificate;

pub(crate) use inlet_listener::*;
pub(crate) use inlet_shared_state::*;
pub(crate) use inlet_sni_root_listener::*;
pub use interceptor::{
    Direction, PortalInletInterceptor, PortalInterceptor, PortalInterceptorFactory,
    PortalInterceptorWorker, PortalOutletInterceptor,
};
pub(crate) use outlet_listener::*;
pub use portal_message::*;
pub(crate) use portal_receiver::*;
pub(crate) use portal_worker::*;
pub use tls_certificate::*;

pub const PSQL_REQUEST_TLS_BIN: [u8; 8] = [0x00, 0x00, 0x00, 0x08, 0x04, 0xd2, 0x16, 0x2f];
pub const PSQL_RESPONSE_TLS_BIN: [u8; 1] = [0x53];
