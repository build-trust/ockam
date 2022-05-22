use bytes::{Buf, BufMut, BytesMut};
use ockam_core::TransportMessage;
use ockam_core::{Decodable, Encodable};
use ockam_transport_core::TransportError;
use tokio_util::codec::{Decoder, Encoder};

pub(crate) struct TransportMessageCodec;

impl Encoder<TransportMessage> for TransportMessageCodec {
    type Error = TransportError;
    fn encode(&mut self, item: TransportMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg_buf = item.encode().map_err(|_| TransportError::SendBadMessage)?;
        let len = msg_buf.len();
        dst.put_u16(len as u16);
        dst.put(&msg_buf[..]);
        Ok(())
    }
}

impl Decoder for TransportMessageCodec {
    type Item = TransportMessage;
    type Error = TransportError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let len = src.get_u16() as usize;
        let msg = TransportMessage::decode(&src.split_to(len)[..])
            .map_err(|_| TransportError::RecvBadMessage)?;

        Ok(Some(msg))
    }
}
