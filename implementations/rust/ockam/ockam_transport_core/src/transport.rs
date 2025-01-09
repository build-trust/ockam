use crate::TransportError;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{async_trait, Address, Encodable, Result, TransportMessage, TransportType};

/// Generic representation of a Transport
/// At minimum, a Transport must be able
///  - return its type
///  - instantiate workers for all the addresses with that transport type in a Route

pub const MAXIMUM_MESSAGE_LENGTH: usize = u16::MAX as usize;

#[async_trait]
pub trait Transport: Send + Sync + 'static {
    /// Return the type of the Transport
    fn transport_type(&self) -> TransportType;

    /// Instantiate transport workers for in order to communicate with a remote address
    /// and return the local address of the transport worker
    async fn resolve_address(&self, address: &Address) -> Result<Address>;

    /// Stop all workers and free all resources associated with the connection
    fn disconnect(&self, address: &Address) -> Result<()>;
}

/// Helper that creates a length-prefixed buffer containing the given
/// `TransportMessage`'s payload
///
/// The length-prefix is encoded as a big-endian 16-bit unsigned
/// integer.
pub fn encode_transport_message(msg: TransportMessage) -> Result<Vec<u8>> {
    let mut msg_buf = msg.encode().map_err(|_| TransportError::SendBadMessage)?;

    if msg_buf.len() > MAXIMUM_MESSAGE_LENGTH {
        Err(TransportError::Capacity)?;
    }

    // Create a buffer that includes the message length in big endian
    let mut len = (msg_buf.len() as u16).to_be_bytes().to_vec();

    // Fun fact: reversing a vector in place, appending the length,
    // and then reversing it again is faster for large message sizes
    // than adding the large chunk of data.
    //
    // https://play.rust-lang.org/?version=stable&mode=release&edition=2018&gist=8669a640004ac85c7be38b19e3e73dcb
    msg_buf.reverse();
    len.reverse();
    msg_buf.append(&mut len);
    msg_buf.reverse();

    Ok(msg_buf)
}

#[cfg(test)]
mod test {
    use super::{encode_transport_message, TransportMessage};
    use ockam_core::route;

    #[test]
    fn prepare_message_should_discard_large_messages() {
        let msg = TransportMessage::latest(route![], route![], vec![0; u16::MAX as usize + 1]);
        let result = encode_transport_message(msg);
        assert!(result.is_err());
    }
}
