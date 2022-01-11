use core::fmt;
use core::str::FromStr;
use ockam_core::compat::string::{String, ToString};
use ockam_core::Result;
use ockam_transport_core::TransportError;

#[derive(Clone, Debug, Default)]
pub struct BleAddr {
    pub device_name: String,
    pub local_name: String,
}

impl fmt::Display for BleAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.local_name)
    }
}

impl From<&BleAddr> for String {
    fn from(other: &BleAddr) -> Self {
        other.to_string()
    }
}

impl FromStr for BleAddr {
    type Err = ockam_core::Error;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        Ok(Self {
            device_name: s.to_string(),
            local_name: s.to_string(),
        })
    }
}

pub fn parse_ble_addr<S: AsRef<str>>(s: S) -> Result<BleAddr> {
    Ok(s.as_ref()
        .parse()
        .map_err(|_| TransportError::InvalidAddress)?)
}
