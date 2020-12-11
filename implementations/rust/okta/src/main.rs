#[macro_use]
mod macros;
mod config;
mod objects;

use config::*;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};
use structopt::StructOpt;

const FILE_NAME: &str = ".env";

fn main() {
    let okta_creds = get_okta_api_info();
}

fn get_okta_api_info() -> Inputs {
    let mut config = Config::from_args();

    if config.is_valid() {
        return config.into();
    }

    if !Path::new(FILE_NAME).is_file() {
        // prompt for input if it wasn't supplied by CLI

        if config.url.is_none() {
            config.url = Some(prompt("Enter okta API URL endpoint ", "[]: ", false));
        }
        if config.token.is_none() {
            config.token = Some(prompt("Enter okta API token", "", true));
        }
        if config.email.is_none() {
            config.email = Some(prompt("Enter okta login email", "[]: ", false));
        }
        if config.password.is_none() {
            config.password = Some(prompt("Enter oka login password", "", true));
        }
    } else {
        let config2: Config =
            serde_json::from_str(&fs::read_to_string(FILE_NAME).unwrap()).unwrap();
        if config.token.is_none() && config2.token.is_some() {
            config.token = config2.token.clone();
        }
        if config.url.is_none() && config2.url.is_some() {
            config.url = config2.url.clone();
        }
        if config.email.is_none() && config2.email.is_some() {
            config.email = config2.email.clone();
        }
        if config.password.is_none() && config2.password.is_some() {
            config.password = config2.password.clone()
        }
    }

    config.into()
}

fn prompt(premsg: &str, p: &str, hide: bool) -> String {
    let mut buffer = String::new();
    loop {
        println!("{}", premsg);

        if hide {
            buffer = rpassword::read_password_from_tty(None)
                .unwrap()
                .as_str()
                .to_string();
        } else {
            print!("{}", p);
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut buffer).unwrap();
            buffer = buffer.trim().to_string();
        }
        if !buffer.is_empty() {
            return buffer;
        }
    }
}
