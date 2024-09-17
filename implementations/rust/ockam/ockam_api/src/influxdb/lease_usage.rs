use minicbor::{CborLen, Decode, Encode};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Clone, Debug, Encode, Decode, CborLen, PartialEq)]
#[rustfmt::skip]
pub enum LeaseUsage {
    #[n(1)] Shared,
    #[n(2)] PerClient,
}

impl FromStr for LeaseUsage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "shared" => Ok(LeaseUsage::Shared),
            "per-client" | "per_client" => Ok(LeaseUsage::PerClient),
            _ => Err(format!("Invalid lease usage: {}", s)),
        }
    }
}

impl Display for LeaseUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeaseUsage::Shared => write!(f, "shared"),
            LeaseUsage::PerClient => write!(f, "per-client"),
        }
    }
}
