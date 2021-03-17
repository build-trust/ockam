pub mod message;
pub mod schema;

pub use message::*;
pub use schema::*;
use std::net::SocketAddr;

pub static DEFAULT_ISSUER_PORT: usize = 7967;
pub static DEFAULT_VERIFIER_PORT: usize = DEFAULT_ISSUER_PORT + 1;

pub fn default_address() -> SocketAddr {
    on("127.0.0.1", DEFAULT_ISSUER_PORT)
}

pub fn on<S: ToString>(host: S, port: usize) -> SocketAddr {
    format!("{}:{}", host.to_string(), port).parse().unwrap()
}

pub fn on_or_default<S: ToString>(host: Option<S>) -> SocketAddr {
    if let Some(host) = host {
        let host = host.to_string();
        if let Some(_) = host.find(":") {
            host.parse().unwrap()
        } else {
            on(host, DEFAULT_ISSUER_PORT)
        }
    } else {
        default_address()
    }
}
