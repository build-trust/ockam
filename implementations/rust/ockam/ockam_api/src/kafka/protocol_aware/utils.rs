use crate::kafka::portal_worker::InterceptError;
use bytes::BytesMut;
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::{Decodable, Encodable, StrBytes};
use std::io::{Error, ErrorKind};

pub(super) fn decode<T, B>(buffer: &mut B, api_version: i16) -> Result<T, InterceptError>
where
    T: Decodable,
    B: ByteBuf,
{
    let response = match T::decode(buffer, api_version) {
        Ok(response) => response,
        Err(_) => {
            warn!("cannot decode kafka message");
            return Err(InterceptError::Io(Error::from(ErrorKind::InvalidData)));
        }
    };
    Ok(response)
}

pub(super) fn encode<H: Encodable, T: Encodable>(
    header: &H,
    body: &T,
    api_version: i16,
) -> Result<BytesMut, InterceptError> {
    let mut buffer = BytesMut::new();

    header
        .encode(&mut buffer, api_version)
        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;
    body.encode(&mut buffer, api_version)
        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

    Ok(buffer)
}

pub(super) fn string_to_str_bytes(ip_address: String) -> StrBytes {
    //TryFrom is broken, ugly but effective
    unsafe { StrBytes::from_utf8_unchecked(bytes::Bytes::from(ip_address)) }
}
