use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Clone, Debug, Deserialize, Serialize, StructOpt)]
pub struct Config {
    #[structopt(short, long)]
    pub api_token: String,
    #[structopt(short = "-i", long)]
    pub client_id: String,
    #[structopt(short = "-s", long)]
    pub client_secret: String,
    #[structopt(short = "-p", long)]
    pub channel_port: u16,
    #[structopt(short = "o", long)]
    pub okta_port: u16,
    #[structopt(short = "-u", long)]
    pub okta_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_token: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            channel_port: 0,
            okta_port: 0,
            okta_url: String::new(),
        }
    }
}
