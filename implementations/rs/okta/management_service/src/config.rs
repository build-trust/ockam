use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Clone, Debug, Deserialize, Serialize, StructOpt)]
pub struct Config {
    #[structopt(short, long)]
    pub api_token: String,
    #[structopt(short="-i", long)]
    pub client_id: String,
    #[structopt(short="-s", long)]
    pub client_secret: String,
    #[structopt(short, long)]
    pub port: u16,
    #[structopt(short, long)]
    pub okta_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self { api_token: String::new(), client_id: String::new(), client_secret: String::new(), port: 0, okta_url: String::new() }
    }
}