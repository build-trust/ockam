use bytes::{Buf, BufMut, BytesMut};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

/// Decoder for length encoded messages.
/// Keeps its internal buffer
pub(super) struct KafkaMessageDecoder {
    buffer: Option<BytesMut>,
    current_message_length: u32,
}

impl KafkaMessageDecoder {
    pub(super) fn new() -> Self {
        Self {
            buffer: Default::default(),
            current_message_length: 0,
        }
    }

    /// Accepts length encoded messages, returns complete messages
    pub(super) fn decode_messages(
        &mut self,
        mut incoming: BytesMut,
        max_message_size: u32,
    ) -> ockam::Result<Vec<BytesMut>> {
        let mut kafka_messages = Vec::new();

        while incoming.remaining() > 0 {
            if self.current_message_length == 0 {
                let current_message_length = incoming.get_u32();
                if current_message_length > max_message_size {
                    return Err(Error::new(
                        Origin::Transport,
                        Kind::Io,
                        "kafka message is bigger than maximum size",
                    ));
                }
                if current_message_length == 0 {
                    return Err(Error::new(
                        Origin::Transport,
                        Kind::Io,
                        "kafka message of size 0",
                    ));
                }
                self.buffer = Some(BytesMut::with_capacity(incoming.remaining()));
                self.current_message_length = current_message_length;
            }

            let missing_bytes =
                self.current_message_length - self.buffer.as_ref().unwrap().len() as u32;
            if missing_bytes as usize > incoming.len() {
                self.buffer.as_mut().unwrap().put_slice(incoming.as_ref());
                incoming.advance(incoming.remaining());
                break;
            } else {
                self.buffer
                    .as_mut()
                    .unwrap()
                    .put_slice(&incoming[0..missing_bytes as usize]);
                incoming.advance(missing_bytes as usize);
                kafka_messages.push(self.buffer.take().unwrap());
                self.current_message_length = 0;
            }
        }

        Ok(kafka_messages)
    }
}

/// Return a length encoded message
pub(super) fn length_encode(content: BytesMut) -> ockam::Result<BytesMut> {
    let mut buffer = BytesMut::new();
    if content.len() >= u32::MAX as usize {
        Err(Error::new(
            Origin::Transport,
            Kind::Io,
            "kafka message is bigger than 4GB",
        ))
    } else {
        buffer.put_u32(content.len() as u32);
        buffer.put_slice(content.as_ref());
        Ok(buffer)
    }
}
