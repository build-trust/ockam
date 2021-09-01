#[macro_use]
extern crate lazy_static;

mod config;

use ockam_vault::{
    ockam_vault_core::{
        SecretAttributes, SecretType, SecretPersistence, Secret, Signer, SecretVault, Hasher
    },
    SoftwareVault,
};
use oktaplugin::{Messages, OckamMessages, OckamRole, KeyEstablishment, EstablishmentBundle};
use rand::prelude::*;
use std::{
    fs,
    io::{self, Write},
    net::TcpStream,
    path::Path,
    sync::Mutex,
};
use serde::Deserialize;
use structopt::StructOpt;
use oktaplugin::Messages::OktaRequest;
use std::collections::BTreeMap;
use std::thread::sleep;

const FILE_NAME: &str = ".env";

lazy_static! {
    static ref VAULT: Mutex<SoftwareVault> = Default::default();
}

fn main() {
    let cfg =
        if Path::new(FILE_NAME).is_file() {
            let contents = fs::read_to_string(FILE_NAME).unwrap();
            serde_json::from_str::<config::Config>(&contents).unwrap()
        } else {
            config::Config::from_args()
        };


    let res = TcpStream::connect(format!("{}:{}", cfg.service_address, cfg.service_port));
    if res.is_err() {
        eprintln!("Unable to connect to service");
        return;
    }
    let mut service_stream = res.unwrap();
    let login_id = thread_rng().gen::<u64>() as usize;
    let msg = Messages::OktaLogin(login_id);
    let res = serde_json::to_writer(&service_stream, &msg);
    if res.is_err() {
        eprintln!("Unable to send login notice");
        service_stream.shutdown(std::net::Shutdown::Both).unwrap();
        return;
    }
    let res = service_stream.flush();
    if res.is_err() {
        eprintln!("Unable to send login notice");
        service_stream.shutdown(std::net::Shutdown::Both).unwrap();
        return;
    }

    let mut services_data = BTreeMap::new();
    let mut x3dh_bundles: BTreeMap<u32, EstablishmentBundle>;
    let mut access_token = String::new();
    let mut credential_key: Option<Secret> = None;

    let mut de = serde_json::Deserializer::from_reader(service_stream.try_clone().unwrap());
    loop {
        let res = Messages::deserialize(&mut de);
        if res.is_err() {
            let err = res.unwrap_err();
            match err.classify() {
                serde_json::error::Category::Eof => {
                    eprintln!("Server closed connection");
                    return;
                }
                _ => {
                    eprintln!("Unknown message type");
                    continue;
                }
            }
        }
        let msg = res.unwrap();
        match msg {
            Messages::OktaLoginUrl{ preamble, url } => {
                println!("{}", preamble);
                println!("{}", url);
            },
            Messages::OktaGrantToken { .. } => {

            },
            Messages::OktaAccessToken { token } => {
                access_token = token;

                if credential_key.is_some() {
                    let msg = OktaRequest { token: access_token.clone(), msg: OckamMessages::ListServicesRequest {
                        limit: 1000,
                        offset: 0
                    }};
                    serde_json::to_writer(&mut service_stream, &msg).unwrap();
                    service_stream.flush().unwrap();
                    continue;
                }

                let mut vault = VAULT.lock().unwrap();

                let res = vault.secret_generate(SecretAttributes::new(SecretType::Curve25519, SecretPersistence::Ephemeral,0));
                if res.is_err() {
                    eprintln!("Couldn't create enroller credential key");
                    service_stream.shutdown(std::net::Shutdown::Both).unwrap();
                    return;
                }
                let secret = res.unwrap();
                let public = vault.secret_public_key_get(&secret).unwrap();
                let proof = vault.sign(&secret, public.as_ref()).unwrap();

                let mut public_key = [0u8; 32];
                public_key.copy_from_slice(public.as_ref());
                let mut proof_buf = [0u8; 64];
                proof_buf.copy_from_slice(proof.as_ref());

                let msg = Messages::OktaRequest {
                    token: access_token.clone(),
                    msg: OckamMessages::BecomeRequest {
                        role: OckamRole::Enroller {
                            public_key,
                            proof: proof_buf,
                        }
                    }
                };
                serde_json::to_writer(&mut service_stream, &msg).unwrap();
                service_stream.flush().unwrap();
                credential_key = Some(secret);
            },
            Messages::OktaResponse { msg } => {
                match msg {
                    OckamMessages::AccessDenied => {
                        let login_id = thread_rng().gen::<u64>() as usize;
                        let msg = Messages::OktaLogin(login_id);
                        serde_json::to_writer(&mut service_stream, &msg).unwrap();
                        service_stream.flush().unwrap();
                    },
                    OckamMessages::BecomeResponse { result, msg } => {
                        if result {
                            println!("Became Enroller successfully");

                            let msg = OktaRequest { token: access_token.clone(), msg: OckamMessages::ListServicesRequest {
                                limit: 1000,
                                offset: 0
                            }};
                            serde_json::to_writer(&mut service_stream, &msg).unwrap();
                            service_stream.flush().unwrap();
                        } else {
                            println!("Become Enroller failure: {:?}", msg);
                            service_stream.shutdown(std::net::Shutdown::Both).unwrap();
                            return;
                        }
                    },
                    OckamMessages::ListServicesResponse { services } => {
                        services_data = services.iter().map(|s| (s.id, s.clone())).collect();
                        let mut bundles = Vec::new();
                        for s in &services {
                            if s.key_establishment.contains(&KeyEstablishment::X3dh) {
                                bundles.push(s.id);
                            }
                        }
                        println!("Services: {:?}", services_data);
                        let req = OktaRequest { token: access_token.clone(), msg: OckamMessages::GetEstablishmentBundlesRequest {
                            services: bundles
                        }};
                        serde_json::to_writer(&mut service_stream, &req).unwrap();
                        service_stream.flush().unwrap();
                    },
                    OckamMessages::GetEstablishmentBundlesResponse { services } => {
                        println!("Received bundles: {:?}", services);

                        x3dh_bundles = services.iter().map(|s| (s.service_id, s.clone())).collect();

                        println!("Start a truck to begin enrollment");
                        let res = TcpStream::connect(format!("{}:{}", cfg.truck_address, cfg.truck_port));
                        while res.is_err() {
                            sleep(std::time::Duration::from_secs(2));
                            println!("Waiting for truck at {}:{}", cfg.truck_address, cfg.truck_port);
                        }

                        let mut truck_stream = res.unwrap();

                        println!("Connected to truck");
                        print!("Starting enrollment...");
                        io::stdout().flush().unwrap();
                        let mut cred_id = [0u8; 16];
                        rand::thread_rng().fill_bytes(&mut cred_id);

                        serde_json::to_writer(&truck_stream, &OckamMessages::BeginDeviceEnrollment {
                            nonce: cred_id
                        }).unwrap();
                        let mut truck_de = serde_json::Deserializer::from_reader(truck_stream.try_clone().unwrap());
                        let res = OckamMessages::deserialize(&mut truck_de);
                        if res.is_err() {
                            println!("fail");
                            println!("{:?}", res.unwrap_err());
                            truck_stream.shutdown(std::net::Shutdown::Both).unwrap();
                            service_stream.shutdown(std::net::Shutdown::Both).unwrap();
                            return;
                        }
                        match res.unwrap() {
                            OckamMessages::DeviceEnrollmentRequest { nonce, blind_device_secret, .. } => {
                                if cred_id != nonce {
                                    println!("fail");
                                    println!("Invalid cred id");
                                    truck_stream.shutdown(std::net::Shutdown::Both).unwrap();
                                    service_stream.shutdown(std::net::Shutdown::Both).unwrap();
                                    return;
                                }
                                println!("pass");
                                print!("Creating credential...");
                                io::stdout().flush().unwrap();
                                let mut vault = VAULT.lock().unwrap();
                                let attributes = vec![
                                    cred_id.to_vec(),
                                    b"Acme Truck".to_vec(),
                                    b"Site B".to_vec(),
                                    b"LTE".to_vec()
                                ];
                                let mut sig_data = Vec::new();
                                sig_data.extend_from_slice(&blind_device_secret);
                                let mut hashed_attributes = Vec::new();
                                hashed_attributes.push(blind_device_secret.to_vec());
                                for a in &attributes {
                                    let hash = vault.sha256(a.as_slice()).unwrap();
                                    sig_data.extend_from_slice(&hash);
                                    hashed_attributes.push(hash.to_vec());
                                }
                                let sig_data = vault.sha256(&sig_data).unwrap();
                                let sig_key = credential_key.as_ref().unwrap();
                                let signature = vault.sign(sig_key, &sig_data).unwrap();
                                serde_json::to_writer(&mut truck_stream, &OckamMessages::DeviceEnrollmentResponse {
                                    schema: services_data[&1].schemas[0].clone(),
                                    service: x3dh_bundles[&1].clone(),
                                    attributes,
                                    attestation: signature.as_ref().to_vec()
                                }).unwrap();
                                println!("done");

                                println!("Closing down");
                                let _ = truck_stream.shutdown(std::net::Shutdown::Both);
                                service_stream.shutdown(std::net::Shutdown::Both).unwrap();
                                break;
                            },
                            _ => {

                            }
                        }
                    }
                    _ => {

                    }
                }
            },
            _ => {

            }
        }
    }
}
