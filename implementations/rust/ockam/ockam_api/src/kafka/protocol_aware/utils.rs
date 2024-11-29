use crate::kafka::protocol_aware::InterceptError;
use bytes::BytesMut;
use kafka_protocol::messages::ApiKey;
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::{Decodable, Encodable};

pub(crate) fn decode_body<T, B>(buffer: &mut B, api_version: i16) -> Result<T, InterceptError>
where
    T: Decodable,
    B: ByteBuf,
{
    let response = match T::decode(buffer, api_version) {
        Ok(response) => response,
        Err(error) => {
            warn!("cannot decode kafka message, closing connection");
            debug!("error: {:?}", error);
            return Err(InterceptError::InvalidData);
        }
    };
    Ok(response)
}

pub(crate) fn encode_request<H: Encodable, T: Encodable>(
    header: &H,
    body: &T,
    api_version: i16,
    api_key: ApiKey,
) -> Result<BytesMut, InterceptError> {
    let mut buffer = BytesMut::new();

    header
        .encode(&mut buffer, api_key.request_header_version(api_version))
        .map_err(|_| InterceptError::InvalidData)?;
    body.encode(&mut buffer, api_version)
        .map_err(|_| InterceptError::InvalidData)?;

    Ok(buffer)
}

pub(crate) fn encode_response<H: Encodable, T: Encodable>(
    header: &H,
    body: &T,
    api_version: i16,
    api_key: ApiKey,
) -> Result<BytesMut, InterceptError> {
    let mut buffer = BytesMut::new();

    header
        .encode(&mut buffer, api_key.response_header_version(api_version))
        .map_err(|_| InterceptError::InvalidData)?;
    body.encode(&mut buffer, api_version)
        .map_err(|_| InterceptError::InvalidData)?;

    Ok(buffer)
}
