#[macro_use]
extern crate arrayref;
#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;
mod config;
mod objects;

use colored::Colorize;
use isahc::prelude::*;
// use objects::UsersInGroup;
use rand::RngCore;
use ockam::{CredentialSchema, CredentialAttributeSchema, CredentialAttributeType};
use ockam_vault::{
    *,
    ockam_vault_core::*,
};
use ockam_vault_sync_core::VaultMutex;
use ockam_key_exchange_core::{NewKeyExchanger, KeyExchanger};
use ockam_key_exchange_x3dh::*;
use oktaplugin::{
    *,
    Messages::OktaResponse,
};
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::{BTreeSet, BTreeMap},
    fs,
    io::{self, stdout, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::Mutex,
    thread,
};
use structopt::StructOpt;
use warp::Filter;
use std::thread::sleep;

const FILE_NAME: &str = ".env";

type WarpResult<T> = Result<T, warp::Rejection>;

lazy_static! {
    static ref STATE: Mutex<RefCell<OktaData>> = Default::default();
    static ref CFG: Mutex<RefCell<config::Config>> = Default::default();
}

#[derive(Debug)]
struct OktaData {
    state: BTreeMap<[u8; 16], StateData>,
    ids: BTreeMap<usize, [u8; 16]>,
    redirect_uri: String
}

impl Default for OktaData {
    fn default() -> Self {
        Self {
            state: BTreeMap::new(),
            ids : BTreeMap::new(),
            redirect_uri: String::new()
        }
    }
}

#[derive(Debug)]
struct StateData {
    id: usize,
    state: [u8; 16],
    stream: TcpStream,
}

// /// Groups to check against
// /// Enroller => 00g26qjjaQlo67l4s5d6
// const OKTA_GROUPS: [&str; 1] = ["00g26qjjaQlo67l4s5d6"];

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
    //pub fn id(&self) -> String {
    //    self.uid.clone().unwrap_or(String::new())
    //}

    pub fn username(&self) -> String {
        self.username.clone().unwrap_or(String::new())
    }

    // /// An example of extracting the scope
    //pub fn scopes(&self) -> BTreeSet<String> {
    //    self.scope.clone().unwrap_or(String::new()).split(',').map(|s| s.to_string()).collect()
    //}
}

async fn parse_query_params(query: OktaOpenIdResponse) -> WarpResult<impl warp::Reply> {
    thread::spawn(|| {
        let lock = STATE.lock().unwrap();
        let state = lock.borrow();
        let lock = CFG.lock().unwrap();
        let cfg = lock.borrow();
        start( &cfg, &state, query);
    });
    Ok("Success. You may close this tab and return to the shell.")
}

fn start(cfg: &config::Config, data: &OktaData, query: OktaOpenIdResponse) {
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
    if !data.state.contains_key(rx_state.as_slice()) {
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
    let res = introspect(&cfg, &creds.access_token);
    if res.is_err() {
        fail("fail");
        eprintln!("Unable to introspect access token from okta: {:?}", res);
        return;
    }
    pass("success");
    let intro = res.unwrap();
    println!("Logged in as {}", intro.username());
    // print!("Checking user roles...");
    // stdout.flush().unwrap();
    // if !is_user_in_group(&cfg, &intro.id(), &OKTA_GROUPS) {
    //     fail("fail");
    //     eprintln!("{} is not in the correct group", intro.username());
    //     return;
    // }
    // pass("success");

    print!("Sending code back to client...");
    stdout.flush().unwrap();
    let mut stream = &data.state[rx_state.as_slice()].stream;
    let msg = Messages::OktaAccessToken { token: creds.access_token };
    let res = serde_json::to_writer(stream, &msg);
    if res.is_err() {
        fail("fail");
        eprintln!("Unable to send code back to client");
        return;
    }
    stream.flush().unwrap();
    pass("done");
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

fn introspect(cfg: &config::Config, access_token: &str) -> Result<OktaIntrospectResponse, String> {
    let mut response = Request::post(format!("{}/oauth2/default/v1/introspect", cfg.okta_url))
        .header("accept", "application/json")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(format!("client_id={}&client_secret={}&token_type_hint=access_token&token={}", cfg.client_id, cfg.client_secret, access_token)).unwrap()
        .send().unwrap();
    let text = response.text().unwrap();
    serde_json::from_str(&text).map_err(|e| format!("{:?}", e))
}

//fn is_user_in_group(cfg: &config::Config, id: &str, groups: &[&str]) -> bool {
//    let mut response = Request::get(format!("{}/api/v1/groups/{}/users", cfg.okta_url, groups[0]))
//        .header("Accept", "application/json")
//        .header("Content-type", "application/json")
//        .header("Authorization", format!("SSWS {}", cfg.api_token))
//        .body("").unwrap()
//        .send().unwrap();
//
//    if response.status() != isahc::http::StatusCode::OK {
//        eprintln!("Unable to read Okta group information");
//        return false;
//    }
//    let text = response.text().unwrap();
//    let response: Vec<UsersInGroup> = serde_json::from_str(&text).unwrap();
//    response.iter().any(|u| u.id == id)
//}

#[cfg(target_os = "windows")]
fn pass(s: &str) {
    println!("{}", s);
}
#[cfg(not(target_os = "windows"))]
fn pass(s: &str) {
    println!("{}", s.green());
}

#[cfg(target_os = "windows")]
fn highlight(s: &str) {
    println!("{}", s);
}
#[cfg(not(target_os = "windows"))]
fn highlight(s: &str) {
    println!("{}", s.yellow());
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

    okta_data.redirect_uri = format!("http://localhost:{}/authorization-code/callback", cfg.okta_port);

    // println!("Open the following URL in a browser to continue");
    // println!("{}/oauth2/default/v1/authorize?response_type=code&client_id={}&redirect_uri={}&state={}&nonce={}&scope=openid",
    //          cfg.okta_url,
    //          percent_encoding::utf8_percent_encode(&cfg.client_id, percent_encoding::NON_ALPHANUMERIC),
    //          percent_encoding::utf8_percent_encode(&okta_data.redirect_uri, percent_encoding::NON_ALPHANUMERIC),
    //          base64_url::encode(&okta_data.state),
    //          base64_url::encode(&nonce));

    *STATE.lock().unwrap().borrow_mut() = okta_data;

    let port = cfg.okta_port;
    *CFG.lock().unwrap().borrow_mut() = cfg;

    thread::spawn(|| {
       channel_listener();
    });

    let callback = warp::get()
        .and(warp::path("authorization-code"))
        .and(warp::path("callback"))
        .and(warp::query())
        .and_then(parse_query_params);
    warp::serve(callback).run(([127,0,0,1], port)).await;
}

fn channel_listener() {
    let mut enrollers: BTreeMap<String, BTreeSet<[u8; 32]>> = BTreeMap::new();
    let cfg = CFG.lock().unwrap().borrow().clone();
    let listener = TcpListener::bind(format!("127.0.0.1:{}", cfg.channel_port)).unwrap();
    let mut xxvault = VaultMutex::create(SoftwareVault::default());
    let x3dh_kex = X3dhNewKeyExchanger::new(xxvault.clone());
    let mut responder = Some(Box::new(x3dh_kex.responder().unwrap()));
    let mut nonce = 2u16;

    let mut connections = Vec::new();

    let mut completed_key_exchange = None;

    loop {
        listener.set_nonblocking(true).unwrap();
        let res = listener.accept();
        if res.is_err() {
            let err = res.unwrap_err();
            match err.kind() {
                io::ErrorKind::WouldBlock => {
                    if connections.is_empty() {
                        sleep(std::time::Duration::from_millis(1000));
                    }
                },
                _ => {
                    eprintln!("{:?}", err);
                }
            }
        } else {
            let (stream, addr) = res.unwrap();
            println!("Connection from {:?}", addr);
            connections.push(stream);
        }

        let mut i = 0;
        while i < connections.len() {
            let mut stream = connections.get_mut(i).unwrap();
            stream.set_nonblocking(true).unwrap();
            let mut de = serde_json::Deserializer::from_reader(stream.try_clone().unwrap());

            let res = Messages::deserialize(&mut de);
            if res.is_err() {
                let err = res.unwrap_err();
                match err.classify() {
                    serde_json::error::Category::Io => {
                        sleep(std::time::Duration::from_millis(1000));
                        i += 1;
                        continue;
                    },
                    serde_json::error::Category::Eof => {
                        eprintln!("Client closed connection");
                        connections.remove(i);
                        continue;
                    }
                    err => {

                        i += 1;
                        eprintln!("Unknown message type: {:?}", err);
                        continue;
                    }
                }
            }

            let m = res.unwrap();
            i += 1;

            match m {
                Messages::OktaLogin(id) => {
                    let cfg = CFG.lock().unwrap().borrow().clone();
                    let lock = STATE.lock().unwrap();
                    let mut okta_data = lock.borrow_mut();

                    let mut nonce = [0u8; 16];
                    rand::thread_rng().fill_bytes(&mut nonce);
                    let mut state = [0u8; 16];

                    if let Some(s) = okta_data.ids.get(&id) {
                        state.copy_from_slice(s);
                    } else {
                        rand::thread_rng().fill_bytes(&mut state);
                        okta_data.ids.insert(id, state);
                        okta_data.state.insert(state, StateData {
                            state,
                            id,
                            stream: stream.try_clone().unwrap()
                        });
                    }

                    let msg = Messages::OktaLoginUrl {
                        preamble: "Open the following URL in a browser to continue".to_string(),
                        url: format!("{}/oauth2/default/v1/authorize?response_type=code&client_id={}&redirect_uri={}&state={}&nonce={}&scope=openid",
                                     cfg.okta_url,
                                     percent_encoding::utf8_percent_encode(&cfg.client_id, percent_encoding::NON_ALPHANUMERIC),
                                     percent_encoding::utf8_percent_encode(&okta_data.redirect_uri, percent_encoding::NON_ALPHANUMERIC),
                                     base64_url::encode(&state),
                                     base64_url::encode(&nonce))
                    };
                    serde_json::to_writer(&mut stream, &msg).unwrap();
                    stream.flush().unwrap();
                },
                Messages::OktaRequest { token, msg } => {
                    println!("Received token = {}\n msg = {:?}", token, msg);
                    // Verify Okta token
                    let lock = CFG.lock().unwrap();
                    let cfg = lock.borrow();
                    let res = introspect(&cfg, &token);
                    if res.is_err() {
                        println!("Invalid access token: {:?}", res);
                        continue;
                    }
                    let introspect_res = res.unwrap();
                    if !introspect_res.active {
                        println!("Access token is not active");
                        continue;
                    }

                    match msg {
                        OckamMessages::BecomeRequest {role} => {
                            match role {
                                OckamRole::Enroller {public_key, proof} => {
                                    // if !is_user_in_group(&cfg, &introspect_res.id(), &OKTA_GROUPS) {
                                    //     println!("Access Denied, no Enroller group associated with user");
                                    //     let msg = OktaResponse {msg : OckamMessages::BecomeResponse { result: false, msg: "Access Denied, no Enroller group associated with user".to_string() } };
                                    //     stream.write(&serde_bare::to_vec(&msg).unwrap()).unwrap();
                                    //     stream.flush().unwrap();
                                    //     continue;
                                    // }
                                    let mut vault = SoftwareVault::default();
                                    let pub_key =  PublicKey::new(public_key.into());
                                    let proof = ockam_vault_core::Signature::new(proof.into());
                                    if vault.verify(&proof, &pub_key, pub_key.as_ref()).is_err() {
                                        println!("Invalid enroller key");
                                        let msg = OktaResponse {msg : OckamMessages::BecomeResponse { result: false, msg: "Invalid enroller key".to_string() } };
                                        serde_json::to_writer(&mut stream, &msg).unwrap();
                                        stream.flush().unwrap();
                                        continue;
                                    }
                                    let msg = OktaResponse {msg : OckamMessages::BecomeResponse { result: true, msg: String::new() } };
                                    serde_json::to_writer(&mut stream, &msg).unwrap();
                                    stream.flush().unwrap();
                                    let uid = introspect_res.uid.clone().unwrap();
                                    if let Some(keys) = enrollers.get_mut(&uid) {
                                        keys.insert(public_key);
                                    } else {
                                        let mut keys = BTreeSet::new();
                                        keys.insert(public_key);
                                        enrollers.insert(uid, keys);
                                    }
                                }
                            }
                        },
                        OckamMessages::ListServicesRequest { limit, offset  } => {
                            println!("List no more than {} services starting at {}", limit, offset);

                            // Ignore limit and offset since its hardcoded
                            let services = vec![OckamService {
                                id: 1,
                                key_establishment: vec![KeyEstablishment::Xx, KeyEstablishment::X3dh],
                                schemas: vec![CredentialSchema {
                                    id: "TruckManagement0001".to_string(),
                                    label: String::new(),
                                    description: String::new(),
                                    attributes: vec![
                                        CredentialAttributeSchema {
                                            label: "device_id".to_string(),
                                            description: String::new(),
                                            unknown: true,
                                            attribute_type: CredentialAttributeType::Blob,
                                        },
                                        CredentialAttributeSchema {
                                            label: "can_access_data".to_string(),
                                            description: "Is allowed to access the server data".to_string(),
                                            unknown: false,
                                            attribute_type: CredentialAttributeType::Number,
                                        }
                                    ]
                                }]
                            }];

                            let msg = OktaResponse { msg: OckamMessages::ListServicesResponse { services }};
                            serde_json::to_writer(&mut stream, &msg).unwrap();
                            stream.flush().unwrap();
                        },
                        OckamMessages::GetEstablishmentBundlesRequest { services } => {
                            println!("GetEstablishmentBundlesRequest: {:?}", services);
                            loop {
                                if let Some(ref mut resp) = responder {
                                    // XXX(thom) is this right?
                                    let res = resp.generate_request(b"").unwrap();
                                    let msg = OktaResponse {
                                        msg: OckamMessages::GetEstablishmentBundlesResponse {
                                            services: vec![EstablishmentBundle {
                                                service_id: 1,
                                                address: format!("127.0.0.1:{}", cfg.channel_port),
                                                key_establishment: KeyEstablishment::X3dh,
                                                key_establishment_data: res
                                            }]
                                        }
                                    };
                                    serde_json::to_writer(&mut stream, &msg).unwrap();
                                    stream.flush().unwrap();
                                    break;
                                } else {
                                    println!("Enrollment bundle already used. Creating new bundle");
                                    responder = Some(Box::from(x3dh_kex.responder().unwrap()));
                                }
                            }
                        },
                        _ => {

                        }
                    };
                },
                Messages::NonOktaRequest(m) => {
                    match m {
                        OckamMessages::ServiceEnrollmentMessage1(data) => {
                            if let Some(ref mut resp) = responder {
                                match resp.handle_response(&data) {
                                    Err(e) => eprintln!("{}", e.to_string()),
                                    Ok(d) => {
                                        serde_json::to_writer(&mut stream, &OckamMessages::ServiceEnrollmentResponse(d)).unwrap();
                                    }
                                }
                            } else {
                                eprintln!("Enrollment bundle already used.");
                                serde_json::to_writer(&mut stream, &OckamMessages::ServiceEnrollmentResponse(vec![])).unwrap();
                            }
                        },
                        OckamMessages::ServiceEnrollmentMessage2(data) => {
                            if let Some(mut resp) = responder.take() {
                                let res = resp.handle_response(&data);
                                match res {
                                    Err(e) => {
                                        eprintln!("{}", e.to_string());
                                        serde_json::to_writer(&mut stream, &OckamMessages::ServiceEnrollmentResponse(vec![0])).unwrap();
                                    },
                                    Ok(_) => {
                                        let kex = resp.finalize().unwrap();
                                        let ctt = xxvault.aead_aes_gcm_encrypt(&kex.encrypt_key(), &[1], &[0u8; 12], &kex.h()[..]).unwrap();
                                        completed_key_exchange = Some(kex);
                                        serde_json::to_writer(&mut stream, &OckamMessages::ServiceEnrollmentResponse(ctt)).unwrap();
                                        responder = None;
                                    }
                                }
                            }
                        },
                        OckamMessages::ServiceEnrollmentMessage3(data) => {
                            let kex = completed_key_exchange.as_ref().unwrap();
                            let mut n= [0u8; 12];
                            n[11] = 1;
                            let res = xxvault.aead_aes_gcm_decrypt(&kex.decrypt_key(), data.as_slice(), &n, &kex.h()[..]);
                            match res {
                                Err(e) => eprintln!("{}", e.to_string()),
                                Ok(plaintext) => {
                                    match serde_json::from_slice::<Attestation>(plaintext.as_slice()) {
                                        Err(e) => eprintln!("{:?}", e),
                                        Ok(attestation) => {
                                            let mut sig_data = Vec::new();
                                            for a in &attestation.attributes {
                                                let hash = xxvault.sha256(a).unwrap();
                                                sig_data.extend_from_slice(&hash);
                                            }
                                            let sig_data = xxvault.sha256(&sig_data).unwrap();
                                            let signature = *array_ref![attestation.signature, 0, 64];
                                            let mut verified = false;
                                            for (_, keys) in &enrollers {
                                                for key in keys {
                                                    if xxvault.verify(&ockam_vault_core::Signature::new(signature.to_vec()), &PublicKey::new(key.to_vec()), &sig_data).is_ok() {
                                                        verified = true;
                                                        break;
                                                    }
                                                }
                                                if verified {
                                                    break;
                                                }
                                            }
                                            n[11] = 2;
                                            let ctt = xxvault.aead_aes_gcm_encrypt(&kex.encrypt_key(), &[verified as u8], &n, &kex.h()[..]).unwrap();

                                            serde_json::to_writer(&mut stream, &OckamMessages::ServiceEnrollmentResponse(ctt)).unwrap();
                                            stream.flush().unwrap();
                                        }
                                    }
                                }
                            }
                        },
                        OckamMessages::GeneralMessage(data) => {
                            nonce = nonce.wrapping_add(1);
                            let mut n = [0u8; 12];
                            n[10..].copy_from_slice(&nonce.to_be_bytes());
                            let kex = completed_key_exchange.as_ref().unwrap();
                            match xxvault.aead_aes_gcm_decrypt(&kex.decrypt_key(), data.as_slice(), &n, &kex.h()[..]) {
                                Err(e) => eprintln!("An error occurred while decrypting: {}", e.to_string()),
                                Ok(s) => {
                                    print!("Received: ");
                                    highlight(std::str::from_utf8(&s).unwrap());
                                },
                            }
                        },
                        _ => {

                        }
                    }
                },
                _ => {

                }
            }
        }
    }
}
