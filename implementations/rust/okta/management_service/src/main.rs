#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;
mod config;
mod objects;

use colored::Colorize;
use isahc::prelude::*;
use objects::UsersInGroup;
use rand::RngCore;
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::BTreeSet,
    fs,
    io::{stdout, Write},
    path::Path,
    sync::Mutex,
    thread,
};
use structopt::StructOpt;
use warp::Filter;

const FILE_NAME: &str = ".env";

type WarpResult<T> = Result<T, warp::Rejection>;

lazy_static! {
    static ref STATE: Mutex<RefCell<OktaData>> = Default::default();
    static ref CFG: Mutex<RefCell<config::Config>> = Default::default();
}

#[derive(Clone, Debug)]
struct OktaData {
    state: [u8; 16],
    redirect_uri: String
}

impl Default for OktaData {
    fn default() -> Self {
        Self {
            state: [0u8; 16],
            redirect_uri: String::new()
        }
    }
}

/// Groups to check against
/// Enroller => 00g26qjjaQlo67l4s5d6
const OKTA_GROUPS: [&str; 1] = ["00g26qjjaQlo67l4s5d6"];

#[derive(Debug, Deserialize)]
struct OktaOpenIdResponse {
    code: String,
    state: String
}

impl Default for OktaOpenIdResponse {
    fn default() -> Self {
        Self { code: String::new(), state: String::new() }
    }
}

#[derive(Debug, Deserialize)]
struct OktaTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: usize,
    scope: String,
    refresh_token: Option<String>,
    id_token: String
}

#[derive(Debug, Deserialize)]
struct OktaIntrospectResponse {
    active: bool,
    aud: Option<String>,
    client_id: Option<String>,
    device_id: Option<String>,
    exp: Option<usize>,
    iat: Option<usize>,
    iss: Option<String>,
    jti: Option<String>,
    nbf: Option<usize>,
    scope: Option<String>,
    sub: Option<String>,
    token_type: Option<String>,
    uid: Option<String>,
    username: Option<String>
}

impl OktaIntrospectResponse {
    pub fn id(&self) -> String {
        self.uid.clone().unwrap_or(String::new())
    }

    pub fn username(&self) -> String {
        self.username.clone().unwrap_or(String::new())
    }

    pub fn scopes(&self) -> BTreeSet<String> {
        self.scope.clone().unwrap_or(String::new()).split(',').map(|s| s.to_string()).collect()
    }
}

async fn parse_query_params(query: OktaOpenIdResponse) -> WarpResult<impl warp::Reply> {
    thread::spawn(move || {
        let cfg = CFG.lock().unwrap().borrow().clone();
        let state = STATE.lock().unwrap().borrow().clone();
        start(cfg, &state, query);
    });
    Ok("Success. You may close this tab and return to the shell.")
}

fn start(cfg: config::Config, data: &OktaData, query: OktaOpenIdResponse) {
    let mut stdout = stdout();
    println!("Received authorization code");
    print!("Decoding...");
    stdout.flush().unwrap();
    let res = base64_url::decode(&query.state);
    if res.is_err() {
        fail("fail");
        eprintln!("An error occurred while decoding the state: {:?}", res);
        return;
    }
    pass("success");
    print!("Verifying expected state...");
    stdout.flush().unwrap();
    let rx_state = res.unwrap();
    if rx_state != data.state {
        fail("fail");
        eprintln!("Expected state does not match the request");
        return;
    }
    pass("match");
    print!("Obtaining access token...");
    stdout.flush().unwrap();
    let res = exchange_code_for_access_token(&cfg, &data, &query.code);
    if res.is_err() {
        fail("fail");
        eprintln!("Unable to get access token from okta: {:?}", res);
        return;
    }
    pass("success");
    print!("Inspecting user information...");
    stdout.flush().unwrap();
    let creds = res.unwrap();
    let res = introspect(&cfg, &creds);
    if res.is_err() {
        fail("fail");
        eprintln!("Unable to introspect access token from okta: {:?}", res);
        return;
    }
    pass("success");
    let intro = res.unwrap();
    println!("Logged in as {}", intro.username());
    print!("Checking user roles...");
    stdout.flush().unwrap();
    if !is_user_in_group(&cfg, &intro, &OKTA_GROUPS) {
        fail("fail");
        eprintln!("{} is not in the correct group", intro.username());
        return;
    }
    pass("success");
}

fn exchange_code_for_access_token(cfg: &config::Config, data: &OktaData, code: &str) -> Result<OktaTokenResponse, String> {
    let mut response = Request::post(format!("{}/oauth2/default/v1/token", cfg.okta_url))
        .header("accept", "application/json")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(format!("client_id={}&client_secret={}&grant_type=authorization_code&code={}&redirect_uri={}", cfg.client_id, cfg.client_secret, code, data.redirect_uri)).unwrap()
        .send().unwrap();
    let text = response.text().unwrap();
    serde_json::from_str(&text).map_err(|e| format!("{:?}", e))
}

fn introspect(cfg: &config::Config, creds: &OktaTokenResponse) -> Result<OktaIntrospectResponse, String> {
    let mut response = Request::post(format!("{}/oauth2/default/v1/introspect", cfg.okta_url))
        .header("accept", "application/json")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(format!("client_id={}&client_secret={}&token_type_hint=access_token&token={}", cfg.client_id, cfg.client_secret, creds.access_token)).unwrap()
        .send().unwrap();
    let text = response.text().unwrap();
    serde_json::from_str(&text).map_err(|e| format!("{:?}", e))
}

fn is_user_in_group(cfg: &config::Config, creds: &OktaIntrospectResponse, groups: &[&str]) -> bool {
    let mut response = Request::get(format!("{}/api/v1/groups/{}/users", cfg.okta_url, groups[0]))
        .header("Accept", "application/json")
        .header("Content-type", "application/json")
        .header("Authorization", format!("SSWS {}", cfg.api_token))
        .body("").unwrap()
        .send().unwrap();

    if response.status() != isahc::http::StatusCode::OK {
        eprintln!("Unable to read Okta group information");
        return false;
    }
    let text = response.text().unwrap();
    let response: Vec<UsersInGroup> = serde_json::from_str(&text).unwrap();
    let id = creds.id();
    response.iter().any(|u| u.id == id)
}

#[cfg(target_os = "windows")]
fn pass(s: &str) {
    println!("{}", s);
}
#[cfg(not(target_os = "windows"))]
fn pass(s: &str) {
    println!("{}", s.green());
}

#[cfg(target_os = "windows")]
fn fail(s: &str) {
    println!("{}", s);
}
#[cfg(not(target_os = "windows"))]
fn fail(s: &str) {
    println!("{}", s.red());
}

#[tokio::main]
async fn main() {
    let cfg =
        if Path::new(FILE_NAME).is_file() {
            let contents = fs::read_to_string(FILE_NAME).unwrap();
            serde_json::from_str::<config::Config>(&contents).unwrap()
        } else {
            config::Config::from_args()
        };

    let mut okta_data = OktaData::default();
    let mut nonce = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut okta_data.state);
    rand::thread_rng().fill_bytes(&mut nonce);

    okta_data.redirect_uri = format!("http://localhost:{}/authorization-code/callback", cfg.port);

    println!("Open the following URL in a browser to continue");
    println!("{}/oauth2/default/v1/authorize?response_type=code&client_id={}&redirect_uri={}&state={}&nonce={}&scope=openid",
             cfg.okta_url,
             percent_encoding::utf8_percent_encode(&cfg.client_id, percent_encoding::NON_ALPHANUMERIC),
             percent_encoding::utf8_percent_encode(&okta_data.redirect_uri, percent_encoding::NON_ALPHANUMERIC),
             base64_url::encode(&okta_data.state),
             base64_url::encode(&nonce));

    *STATE.lock().unwrap().borrow_mut() = okta_data;

    let port = cfg.port;
    *CFG.lock().unwrap().borrow_mut() = cfg;

    let callback = warp::get()
        .and(warp::path("authorization-code"))
        .and(warp::path("callback"))
        .and(warp::query())
        .and_then(parse_query_params);
    warp::serve(callback).run(([127,0,0,1], port)).await;
}


