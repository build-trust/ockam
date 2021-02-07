/// Represents the failures that can occur in
/// Ockam Connection and Listener traits
#[derive(Clone, Copy, Debug)]
pub enum Error {
    None,
    PeerNotFound,
    Accept,
    Bind,
    AlreadyConnected,
    NotConnected,
    ConnectFailed,
    CheckConnection,
    ReceiveFailed,
}

impl Error {
    /// Error domain
    pub const ERROR_DOMAIN: &'static str = "TRANSPORT_ERROR_DOMAIN";
}

impl Into<ockam_core::Error> for Error {
    fn into(self) -> ockam_core::Error {
        ockam_core::Error::new(self as u32, Error::ERROR_DOMAIN)
    }
}
