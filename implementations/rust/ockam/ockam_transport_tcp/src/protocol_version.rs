use ockam_transport_core::TransportError;

/// TCP Protocol version
#[repr(u8)]
#[derive(Debug)]
pub enum TcpProtocolVersion {
    /// Version 1
    V1 = 1,
}

impl From<TcpProtocolVersion> for u8 {
    fn from(value: TcpProtocolVersion) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for TcpProtocolVersion {
    type Error = ockam_core::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(TcpProtocolVersion::V1),
            _ => Err(TransportError::InvalidProtocolVersion)?,
        }
    }
}
