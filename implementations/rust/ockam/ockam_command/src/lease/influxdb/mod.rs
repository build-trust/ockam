mod create;
mod list;
mod show;

use std::fmt::{self, Display, Formatter};

use clap::ValueEnum;
pub use create::InfluxDbCreateCommand;
pub use list::InfluxDbListCommand;
pub use show::InfluxDbShowCommand;

#[derive(Clone, Debug, Copy, ValueEnum)]
pub enum InfluxDbTokenStatus {
    Active,
    Inactive,
}

impl Display for InfluxDbTokenStatus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(match self {
            InfluxDbTokenStatus::Active => "active",
            InfluxDbTokenStatus::Inactive => "inactive",
        })
    }
}
