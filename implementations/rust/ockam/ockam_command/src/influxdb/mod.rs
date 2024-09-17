use std::str::FromStr;

pub mod inlet;
pub mod outlet;

#[derive(Clone, Debug, PartialEq)]
pub enum LeaseUsage {
    Shared,
    PerClient,
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
