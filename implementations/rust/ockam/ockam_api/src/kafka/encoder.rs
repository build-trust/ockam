use bytes::{Bytes, BytesMut};
use futures::SinkExt;
use tokio::io::{AsyncReadExt, DuplexStream};
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};

use ockam::compat::tokio;
use ockam_core::compat::io::ErrorKind;

use crate::kafka::portal_worker::MAX_KAFKA_MESSAGE_SIZE;

/// internal util, pass through to encode length delimited kafka packages
/// keeps its own internal buffer
pub(crate) struct KafkaEncoder {
    read_half: DuplexStream,
    framed_write_half: FramedWrite<DuplexStream, LengthDelimitedCodec>,
}

impl KafkaEncoder {
    pub(crate) fn new() -> Self {
        //should be as big as the biggest kafka message we support, 4 bytes are the length 32bit field
        //we assume each cycle we write a kafka message and then immediately empty the buffer
        let (write_half, read_half) = tokio::io::duplex(MAX_KAFKA_MESSAGE_SIZE + 4);
        Self {
            read_half,
            framed_write_half: FramedWrite::new(
                write_half,
                LengthDelimitedCodec::builder()
                    .max_frame_length(MAX_KAFKA_MESSAGE_SIZE)
                    .length_field_length(4)
                    .new_codec(),
            ),
        }
    }

    pub(crate) async fn write_kafka_message(
        &mut self,
        kafka_message: Vec<u8>,
    ) -> Result<(), ockam_core::compat::io::Error> {
        self.framed_write_half
            .send(Bytes::from(kafka_message))
            .await?;
        self.framed_write_half.flush().await?;
        Ok(())
    }

    pub(crate) async fn read_length_encoded(
        &mut self,
    ) -> Result<Vec<u8>, ockam_core::compat::io::Error> {
        let mut buffer = BytesMut::with_capacity(1024);
        let read = self.read_half.read_buf(&mut buffer).await?;
        if read == 0 {
            //we should never read 0 bytes since at least one kafka message must
            //be written before calling this method
            Err(ockam_core::compat::io::Error::from(ErrorKind::BrokenPipe))
        } else {
            Ok(buffer.to_vec())
        }
    }
}
