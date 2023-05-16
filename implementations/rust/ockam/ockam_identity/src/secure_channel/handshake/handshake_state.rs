use crate::secure_channel::handshake::constants::SHA256_SIZE_USIZE;
use crate::secure_channel::handshake::handshake_state::Status::Initial;
use crate::Identity;
use arrayref::array_ref;
use ockam_core::vault::SecretType::X25519;
use ockam_core::vault::{KeyId, PublicKey};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub(super) struct HandshakeState {
    pub(super) s: KeyId,
    pub(super) e: KeyId,
    pub(super) k: KeyId,
    pub(super) re: PublicKey,
    pub(super) rs: PublicKey,
    pub(super) n: usize,
    pub(super) h: [u8; SHA256_SIZE_USIZE],
    pub(super) ck: KeyId,
    pub(super) prologue: Vec<u8>,
    pub(super) message1_payload: Vec<u8>,
    pub(super) identity_payload: Vec<u8>,
    pub(super) status: Status,
}

impl HandshakeState {
    pub(super) fn new(s: KeyId, e: KeyId, identity_payload: Vec<u8>) -> HandshakeState {
        HandshakeState {
            s,
            e,
            k: "".to_string(),
            re: PublicKey::new(vec![], X25519),
            rs: PublicKey::new(vec![], X25519),
            n: 0,
            h: [0u8; SHA256_SIZE_USIZE],
            ck: "".to_string(),
            prologue: vec![],
            message1_payload: vec![],
            identity_payload,
            status: Initial,
        }
    }

    pub(super) fn mix_hash(&mut self, data: &[u8]) {
        let mut input = self.h.to_vec();
        input.extend(data);
        self.h = Self::sha256(&input)
    }

    fn sha256(data: &[u8]) -> [u8; 32] {
        let digest = Sha256::digest(data);
        *array_ref![digest, 0, 32]
    }
}

#[derive(Debug, Clone)]
pub(super) enum Status {
    Initial,
    WaitingForMessage1,
    WaitingForMessage2,
    WaitingForMessage3,
    Ready(HandshakeResults),
}

#[derive(Debug, Clone)]
pub(super) struct HandshakeResults {
    pub(super) their_identity: Identity,
    pub(super) encryption_key: KeyId,
    pub(super) decryption_key: KeyId,
}
