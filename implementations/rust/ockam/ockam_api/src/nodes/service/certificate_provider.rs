use crate::nodes::NodeManager;
use minicbor::{Decode, Decoder, Encode};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Any, NeutralMessage, Routed};
use ockam_multiaddr::MultiAddr;
use ockam_node::{Context, MessageSendReceiveOptions};
use ockam_transport_tcp::{TlsCertificate, TlsCertificateProvider};
use std::fmt::{Debug, Display, Formatter};
use std::sync::Weak;
use std::time::Duration;
use tonic::async_trait;

#[derive(Clone)]
pub(crate) struct ProjectCertificateProvider {
    node_manager: Weak<NodeManager>,
    to: MultiAddr,
}

impl Debug for ProjectCertificateProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProjectCertificateProvider {{ to: {:?} }}", self.to)
    }
}

impl ProjectCertificateProvider {
    pub fn new(node_manager: Weak<NodeManager>, to: MultiAddr) -> Self {
        Self { node_manager, to }
    }
}

impl Display for ProjectCertificateProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "certificate retrieval from: {}", self.to)
    }
}

#[derive(Encode, Decode)]
struct CertificateRequest {}

#[derive(Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
struct CertificateResponse {
    #[n(1)] kind: ReplyKind,
    #[n(2)] certificate: Option<TlsCertificate>,
}

#[derive(Debug, Encode, Decode, PartialEq)]
#[rustfmt::skip]
#[cbor(index_only)]
enum ReplyKind {
    #[n(0)] Ready,
    #[n(1)] NotReady,
    #[n(2)] Unsupported,
}

#[async_trait]
impl TlsCertificateProvider for ProjectCertificateProvider {
    async fn get_certificate(&self, context: &Context) -> ockam_core::Result<TlsCertificate> {
        debug!("requesting TLS certificate from: {}", self.to);
        let node_manager = self.node_manager.upgrade().ok_or_else(|| {
            ockam_core::Error::new(Origin::Transport, Kind::Invalid, "NodeManager shut down")
        })?;
        let connection = {
            node_manager
                .make_connection(
                    context,
                    &self.to,
                    node_manager.node_identifier.clone(),
                    None,
                    None,
                )
                .await?
        };

        let options = MessageSendReceiveOptions::new().with_timeout(Duration::from_secs(30));

        let payload = {
            let mut buffer = Vec::new();
            minicbor::Encoder::new(&mut buffer).encode(&CertificateRequest {})?;
            buffer
        };

        let reply: Routed<Any> = context
            .send_and_receive_extended(connection.route()?, NeutralMessage::from(payload), options)
            .await?;

        let payload = reply.into_payload();
        let reply: CertificateResponse = Decoder::new(&payload).decode()?;

        match reply.kind {
            ReplyKind::Ready => {
                if let Some(certificate) = reply.certificate {
                    Ok(certificate)
                } else {
                    Err(ockam_core::Error::new(
                        Origin::Transport,
                        Kind::Invalid,
                        "invalid reply from certificate provider",
                    ))
                }
            }
            ReplyKind::Unsupported => Err(ockam_core::Error::new(
                Origin::Transport,
                Kind::NotReady,
                "certificate",
            )),
            ReplyKind::NotReady => Err(ockam_core::Error::new(
                Origin::Transport,
                Kind::NotReady,
                "certificate",
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_orchestrator_encoding_not_ready() {
        let payload = hex::decode("A10101").unwrap();
        let reply: CertificateResponse = Decoder::new(&payload).decode().unwrap();
        assert_eq!(reply.kind, ReplyKind::NotReady);
        assert!(reply.certificate.is_none());
    }

    #[test]
    fn check_orchestrator_encoding_ready() {
        let payload =
            hex::decode("a2010002a2014a66756c6c5f636861696e024b707269766174655f6b6579").unwrap();
        let reply: CertificateResponse = Decoder::new(&payload).decode().unwrap();
        assert_eq!(reply.kind, ReplyKind::Ready);
        assert_eq!(
            reply.certificate,
            Some(TlsCertificate {
                full_chain_pem: "full_chain".as_bytes().to_vec(),
                private_key_pem: "private_key".as_bytes().to_vec()
            })
        );
    }
}
