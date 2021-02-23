use ockam::Error;

/// Represents the failures that can occur in
/// Ockam Connection and Listener traits
#[derive(Clone, Copy, Debug)]
pub enum TransportError {
    None,
    PeerNotFound,
    Accept,
    Bind,
    AlreadyConnected,
    NotConnected,
    ConnectFailed,
    CheckConnection,
    ReceiveFailed,
    ConnectionClosed,
    IllFormedMessage,
    BufferTooSmall,
}

pub enum ChannelError {
    None,
    KeyExchange,
}

impl TransportError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 15_000;
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_TRANSPORT";
}

impl Into<Error> for TransportError {
    fn into(self) -> Error {
        Error::new(Self::DOMAIN_CODE + (self as u32), Self::DOMAIN_NAME)
    }
}

impl ChannelError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 16_000;
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_SECURE_CHANNEL";
}

impl Into<Error> for ChannelError {
    fn into(self) -> Error {
        Error::new(Self::DOMAIN_CODE + (self as u32), Self::DOMAIN_NAME)
    }
}
