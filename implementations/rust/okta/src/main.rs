#[macro_use]
mod macros;
mod config;
mod objects;

use config::*;
use isahc::http::StatusCode;
use isahc::prelude::*;
use objects::*;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};
use structopt::StructOpt;

const FILE_NAME: &str = ".env";
/// Groups to check against
/// Enroller => 00g26qjjaQlo67l4s5d6
const OKTA_GROUPS: [&str; 1] = ["00g26qjjaQlo67l4s5d6"];

fn main() {
    let okta_creds = get_okta_api_info();
    let session_creds = okta_login(&okta_creds);
    println!(
        "Session token is valid: {}",
        is_valid_session_token(&okta_creds, &session_creds)
    );
    println!(
        "User in Enroller: {}",
        is_user_in_group(&okta_creds, &session_creds, &OKTA_GROUPS)
    );
}

fn is_valid_session_token(inputs: &Inputs, creds: &Credentials) -> bool {
    let mut response = Request::post(format!("{}/api/v1/sessions/me", inputs.url))
        .header("Content-type", "application/x-www-form-urlencoded")
        .body(format!(
            r#"token={}&token_type_hint=access_token"#,
            creds.session_token
        ))
        .unwrap()
        .send()
        .unwrap();

    if response.status() != StatusCode::OK {
        eprintln!("Unable to check token validity");
        std::process::exit(1);
    }
    let res = response.text().unwrap();
    println!("introspect: {}", res);
    let response: TokenCheck = serde_json::from_str(&res).unwrap();
    response.active
}

fn is_user_in_group(inputs: &Inputs, creds: &Credentials, groups: &[&str]) -> bool {
    let mut response = Request::get(format!("{}/api/v1/groups/{}/users", inputs.url, groups[0]))
        .header("Accept", "application/json")
        .header("Content-type", "application/json")
        .header("Authorization", format!("SSWS {}", inputs.token))
        .body("")
        .unwrap()
        .send()
        .unwrap();

    if response.status() != StatusCode::OK {
        eprintln!("Unable to read Okta group information");
        std::process::exit(1);
    }
    let res = response.text().unwrap();
    let response: Vec<UsersInGroup> = serde_json::from_str(&res).unwrap();
    response.iter().any(|u| u.id == creds.id)
}

fn okta_login(inputs: &Inputs) -> Credentials {
    let mut response = Request::post(format!("{}/api/v1/authn", inputs.url))
        .header("Accept", "application/json")
        .header("Content-type", "application/json")
        .body(format!(
            r#"{{"username":"{}","password":"{}"}}"#,
            inputs.email, inputs.password
        ))
        .unwrap()
        .send()
        .unwrap();
    if response.status() != StatusCode::OK {
        eprintln!("Invalid okta credentials");
        std::process::exit(1);
    }
    let res = response.text().unwrap();
    let response: LoginAttempt = serde_json::from_str(&res).unwrap();

    Credentials {
        id: response.embedded.user.id,
        session_token: response.session_token,
    }
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

fn prompt(pre_msg: &str, p: &str, hide: bool) -> String {
    let mut buffer = String::new();
    loop {
        println!("{}", pre_msg);

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
