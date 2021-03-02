use ockam::{
    CredentialAttributeSchema, CredentialAttributeType, CredentialFragment2, CredentialOffer,
    CredentialPresentation, CredentialRequest, CredentialSchema, PresentationManifest, SECRET_ID,
};
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

big_array! {
    FixedArray;
    48, 96,
}

pub enum Listener {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    Unix(UnixListener),
    Tcp(TcpListener),
}

impl Listener {
    pub fn bind(socket: Option<PathBuf>, port: Option<usize>) -> Result<Self, String> {
        match (socket, port) {
            (_, Some(port)) => {
                println!("Listening on 127.0.0.1:{}", port);
                Ok(Listener::Tcp(
                    TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap(),
                ))
            }
            (Some(socket), None) => {
                if socket.as_path().exists() && std::fs::remove_file(&socket).is_err() {
                    return Err("Socket already in use".to_string());
                }
                println!("Listening on {:?}", socket.clone());
                Ok(Listener::Unix(UnixListener::bind(socket).unwrap()))
            }
            (None, None) => {
                let mut socket = PathBuf::new();
                socket.push("issuer.socket");
                if socket.as_path().exists() && std::fs::remove_file(&socket).is_err() {
                    return Err("Socket already in use".to_string());
                }
                println!("Listening on {:?}", socket.clone());
                Ok(Listener::Unix(UnixListener::bind("issuer.socket").unwrap()))
            }
        }
    }

    pub fn accept(&self) -> Stream {
        match self {
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            Listener::Unix(u) => {
                let (stream, _) = u.accept().unwrap();
                Stream::Unix(stream)
            }
            Listener::Tcp(t) => {
                let (stream, _) = t.accept().unwrap();
                Stream::Tcp(stream)
            }
        }
    }
}

pub enum Stream {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl Stream {
    pub fn try_clone(&self) -> std::io::Result<Self> {
        match self {
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            Stream::Unix(u) => Ok(Self::Unix(u.try_clone()?)),
            Stream::Tcp(t) => Ok(Self::Tcp(t.try_clone()?)),
        }
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            &mut Stream::Unix(ref mut u) => u.read(buf),
            &mut Stream::Tcp(ref mut t) => t.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            Stream::Unix(ref mut u) => u.write(buf),
            Stream::Tcp(ref mut t) => t.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            Stream::Unix(ref mut u) => u.flush(),
            Stream::Tcp(ref mut t) => t.flush(),
        }
    }
}

/// Messages that involve credential issuance and proving
#[derive(Debug, Deserialize, Serialize)]
pub enum CredentialMessage {
    CredentialConnection,
    NewCredential,
    CredentialIssuer {
        #[serde(with = "FixedArray")]
        public_key: [u8; 96],
        #[serde(with = "FixedArray")]
        proof: [u8; 48],
    },
    CredentialOffer(CredentialOffer),
    CredentialRequest(CredentialRequest),
    InvalidCredentialRequest,
    CredentialResponse(CredentialFragment2),
    PresentationManifest(PresentationManifest),
    Presentation(Vec<CredentialPresentation>),
}

pub fn example_schema() -> CredentialSchema {
    CredentialSchema {
        id: "file:///truck-schema-20210227-1_0_0".to_string(),
        label: "Truck Management".to_string(),
        description: "A Demoable schema".to_string(),
        attributes: vec![
            CredentialAttributeSchema {
                label: SECRET_ID.to_string(),
                description: "A unique identifier for maintenance worker. ".to_string(),
                attribute_type: CredentialAttributeType::Blob,
                unknown: true,
            },
            CredentialAttributeSchema {
                label: "can_access".to_string(),
                description: "Can worker access the truck maintenance codes?".to_string(),
                attribute_type: CredentialAttributeType::Number,
                unknown: false,
            },
        ],
    }
}
