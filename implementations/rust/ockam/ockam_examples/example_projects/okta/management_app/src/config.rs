use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Clone, Debug, Deserialize, Serialize, StructOpt)]
pub struct Config {
    #[structopt(short = "-a", long)]
    pub service_address: String,
    #[structopt(short = "-p", long)]
    pub service_port: u16,
    #[structopt(short = "-t", long)]
    pub truck_address: String,
    #[structopt(short = "-u", long)]
    pub truck_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            service_address: "127.0.0.1".to_string(),
            service_port: 0,
            truck_address: "127.0.0.1".to_string(),
            truck_port: 0,
        }
    }
}
