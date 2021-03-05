use ockam::*;
use ockam_vault::{
    SoftwareVault,
    ockam_vault_core::*,
};
use ockam_key_exchange_core::{NewKeyExchanger, KeyExchanger};
use ockam_key_exchange_x3dh::*;
use oktaplugin::*;
use rand::prelude::*;
use std::{
    io::{self, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex}
};
use colored::Colorize;


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

#[derive(Debug, Clone)]
struct Service {
    schema: CredentialSchema,
    bundle: EstablishmentBundle,
}

fn main() {
    let mut id = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut id);

    let service_info;
    let attestation_info;

    let listener = TcpListener::bind("127.0.0.1:8081").unwrap();
    let vault = Arc::new(Mutex::new(SoftwareVault::default()));

    let res = listener.accept();
    if res.is_err() {
        eprintln!("Unable to connect");
        return;
    }
    let (mut stream, addr) = res.unwrap();
    println!("Connection from {:?}", addr);
    loop {
        match serde_bare::from_reader::<&TcpStream, OckamMessages>(&stream) {
            Err(e) => match e {
                serde_bare::Error::Io(err) => match err.kind() {
                    std::io::ErrorKind::UnexpectedEof => {
                        eprintln!("Client closed connection");
                        return;
                    },
                    _ => {
                        eprintln!("Unknown message type");
                        stream.shutdown(std::net::Shutdown::Both).unwrap();
                        return;
                    }
                },
                _ => {
                    eprintln!("Unknown message type");
                    stream.shutdown(std::net::Shutdown::Both).unwrap();
                    return;
                }
            },
            Ok(m) => {
                match m {
                    OckamMessages::BeginDeviceEnrollment { nonce } => {
                        //Just demo signing, nonce is the credential id
                        let v = vault.lock().unwrap();
                        let device_id = v.sha256(&id).unwrap();
                        let msg = OckamMessages::DeviceEnrollmentRequest {
                            nonce,
                            blind_device_secret: device_id,
                            proof_of_secret: [0u8; 32]
                        };
                        serde_bare::to_writer(&mut stream, &msg).unwrap();
                        stream.flush().unwrap();
                    },
                    OckamMessages::DeviceEnrollmentResponse {schema, service, attributes, attestation} => {
                        service_info = Some(Service {
                            schema,
                            bundle: service
                        });
                        attestation_info = Some(Attestation {
                            attributes,
                            signature: attestation
                        });
                        println!("Received enrollment bundle");
                        println!("Closing connection to enroller");
                        let _  = stream.shutdown(std::net::Shutdown::Both);
                        break;
                    }
                    mm => {
                        eprintln!("Unhandled message type: {:?}", mm);
                    }
                }
            }
        }
    }

    let service = service_info.unwrap();

    print!("Connecting to service...");
    io::stdout().flush().unwrap();
    let res = TcpStream::connect(&service.bundle.address);
    if res.is_err() {
        fail("fail");
        return;
    }
    pass("success");
    let mut stream = res.unwrap();
    let x3dh_kex = X3dhNewKeyExchanger::new(vault.clone(), vault.clone());
    let mut initiator = Box::new(x3dh_kex.initiator());
    let prekey = initiator.process(b"").unwrap();

    print!("Sending service enrollment message 1...");
    io::stdout().flush().unwrap();
    let enroll_msg = Messages::NonOktaRequest(OckamMessages::ServiceEnrollmentMessage1(prekey));
    serde_bare::to_writer(&mut stream, &enroll_msg).unwrap();
    stream.flush().unwrap();

    let res = serde_bare::from_reader::<&TcpStream, OckamMessages>(&stream);
    if res.is_err() {
        fail("fail");
        fail(&format!("{:?}", res.unwrap_err()));
        stream.shutdown(std::net::Shutdown::Both).unwrap();
        return;
    }
    pass("success");

    let ciphertext_and_tag = initiator.process(service.bundle.key_establishment_data.as_slice()).unwrap();
    let enroll_msg = Messages::NonOktaRequest(OckamMessages::ServiceEnrollmentMessage2(ciphertext_and_tag));

    print!("Sending service enrollment message 2...");
    io::stdout().flush().unwrap();
    serde_bare::to_writer(&mut stream, &enroll_msg).unwrap();

    let res = serde_bare::from_reader::<&TcpStream, OckamMessages>(&stream);
    if res.is_err() {
        fail("fail");
        fail(&format!("{:?}", res.unwrap_err()));
        stream.shutdown(std::net::Shutdown::Both).unwrap();
        return;
    }
    let completed_key_exchange = initiator.finalize().unwrap();
    let mut vv = vault.lock().unwrap();
    match res.unwrap() {
        OckamMessages::ServiceEnrollmentResponse(data) => {
            match vv.aead_aes_gcm_decrypt(&completed_key_exchange.decrypt_key(), data.as_slice(), &[0u8; 12], &completed_key_exchange.h()[..]) {
                Err(e) => {
                    fail("fail");
                    println!("Unable to decrypt message: {}", e.to_string());
                    stream.shutdown(std::net::Shutdown::Both).unwrap();
                    return;
                },
                Ok(plaintext) => {
                    if plaintext.len() == 1 && plaintext[0] == 1u8 {
                        pass("success");
                    } else {
                        fail("fail");
                        println!("Unable to enroll");
                        stream.shutdown(std::net::Shutdown::Both).unwrap();
                        return;
                    }
                }
            }
        },
        _ => {
            fail("fail");
            println!("Unexpected response");
            stream.shutdown(std::net::Shutdown::Both).unwrap();
            return;
        }
    }

    print!("Sending credential proof to match schema {:?}...", service.schema);
    io::stdout().flush().unwrap();
    let mut attestation = attestation_info.unwrap();
    attestation.attributes.insert(0, id.to_vec());
    let plaintext = serde_bare::to_vec(&attestation).unwrap();
    let mut nonce = [0u8; 12];
    nonce[11] = 1;
    let ciphertext_and_tag = vv.aead_aes_gcm_encrypt(
        &completed_key_exchange.encrypt_key(), plaintext.as_slice(), &nonce, &completed_key_exchange.h()[..]).unwrap();
    serde_bare::to_writer(&mut stream, &Messages::NonOktaRequest(OckamMessages::ServiceEnrollmentMessage3(ciphertext_and_tag))).unwrap();
    stream.flush().unwrap();

    let res = serde_bare::from_reader::<&TcpStream, OckamMessages>(&stream);

    if res.is_err() {
        fail("fail");
        eprintln!("{:?}", res.unwrap_err());
        stream.shutdown(std::net::Shutdown::Both).unwrap();
        return;
    }

    nonce[11] = 2;
    match res.unwrap() {
        OckamMessages::ServiceEnrollmentResponse(data) => {
            match vv.aead_aes_gcm_decrypt(&completed_key_exchange.decrypt_key(), data.as_slice(), &nonce, &completed_key_exchange.h()[..]) {
                Err(e) => {
                    fail("fail");
                    println!("Unable to decrypt message: {}", e.to_string());
                    stream.shutdown(std::net::Shutdown::Both).unwrap();
                    return;
                },
                Ok(plaintext) => {
                    if plaintext.len() == 1 && plaintext[0] == 1u8 {
                        pass("success");
                    } else {
                        fail("fail");
                        println!("Proof validation failed");
                        stream.shutdown(std::net::Shutdown::Both).unwrap();
                        return;
                    }
                }
            }
        },
        _ => {
            fail("fail");
            println!("Unexpected response");
            stream.shutdown(std::net::Shutdown::Both).unwrap();
            return;
        }
    }

    println!("Successfully enrolled to service. Type data to send to the service");
    let mut buffer = String::new();
    let mut nonce = 2u16;
    loop {
        io::stdin().read_line(&mut buffer).unwrap();
        let text = buffer.trim();
        if !text.is_empty() {
            nonce = nonce.wrapping_add(1);
            let mut n = [0u8; 12];
            n[10..].copy_from_slice(&nonce.to_be_bytes());

            let ctt = vv.aead_aes_gcm_encrypt(&completed_key_exchange.encrypt_key(), text.as_bytes(), &n, &completed_key_exchange.h()[..]).unwrap();
            serde_bare::to_writer(&mut stream, &Messages::NonOktaRequest(OckamMessages::GeneralMessage(ctt))).unwrap();
            stream.flush().unwrap();
            buffer = String::new();
        }
    }
}
