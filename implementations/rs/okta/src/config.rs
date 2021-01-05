use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Credentials {
    pub id: String,
    pub session_token: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Inputs {
    pub email: String,
    pub token: String,
    pub url: String,
    pub password: String,
}

impl From<Config> for Inputs {
    fn from(c: Config) -> Self {
        Self {
            email: c.email.unwrap(),
            password: c.password.unwrap(),
            token: c.token.unwrap(),
            url: c.url.unwrap(),
        }
    }
}

#[derive(Debug, StructOpt, Deserialize, Serialize)]
pub struct Config {
    #[structopt(short, long)]
    pub token: Option<String>,
    #[structopt(short, long)]
    pub url: Option<String>,
    #[structopt(short, long)]
    pub email: Option<String>,
    #[structopt(short, long)]
    pub password: Option<String>,
}

impl Config {
    pub fn is_valid(&self) -> bool {
        self.token.is_some()
            && self.url.is_some()
            && self.email.is_some()
            && self.password.is_some()
    }
}
