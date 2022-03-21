use ockam_core::compat::collections::VecDeque;
use ockam_core::compat::vec::Vec;

/// The possible tokens are:
///
/// "e": The sender generates a new ephemeral key pair and stores it in the e variable,
/// writes the ephemeral public key as cleartext into the message buffer,
/// and hashes the public key along with the old h to derive a new h.
///
/// "s": The sender writes its static public key from the s variable into the message buffer,
/// encrypting it if k is non-empty, and hashes the output along with the old h to derive a new h.
///
/// "ee", "se", "es", "ss": A DH is performed between
/// the initiator's key pair (whether static or ephemeral is determined by the first letter)
/// and the responder's key pair (whether static or ephemeral is determined by the second letter).
/// The result is hashed along with the old ck to derive a new ck and k, and n is set to zero.
#[allow(non_camel_case_types, missing_docs)]
#[derive(Debug)]
pub enum PatternToken {
    e,
    s,
    ee,
    es,
    se,
    ss,
}

/// A pre-message pattern is one of the following sequences of tokens:
///
/// "e"
/// "s"
/// "e, s"
/// empty
///
/// The pre-messages represent an exchange of public keys that was somehow performed
/// prior to the handshake, so these public keys must be inputs to Initialize()
/// for the "recipient" of the pre-message.
#[allow(non_camel_case_types, missing_docs)]
pub enum PreMessagePattern {
    e,
    s,
    es,
    Empty,
}

/// A handshake pattern consists of:
///
/// A pre-message pattern for the initiator, representing information about the initiator's
/// public keys that is known to the responder.
///
/// A pre-message pattern for the responder, representing information about the responder's
/// public keys that is known to the initiator.
///
/// A sequence of message patterns for the actual handshake messages.
pub struct HandshakePattern {
    pub(crate) initiator_pre_msg: PreMessagePattern,
    pub(crate) responder_pre_msg: PreMessagePattern,
    pub(crate) message_patterns: VecDeque<Vec<PatternToken>>,
}

impl HandshakePattern {
    /// Create new Pattern
    pub fn new(
        initiator_pre_msg: PreMessagePattern,
        responder_pre_msg: PreMessagePattern,
        message_patterns: VecDeque<Vec<PatternToken>>,
    ) -> Self {
        Self {
            initiator_pre_msg,
            responder_pre_msg,
            message_patterns,
        }
    }

    /// Create XX Pattern
    pub fn new_xx() -> Self {
        let m1 = vec![PatternToken::e].into_iter().collect();
        let m2 = vec![
            PatternToken::e,
            PatternToken::ee,
            PatternToken::s,
            PatternToken::es,
        ]
        .into_iter()
        .collect();
        let m3 = vec![PatternToken::s, PatternToken::se]
            .into_iter()
            .collect();

        Self::new(
            PreMessagePattern::Empty,
            PreMessagePattern::Empty,
            vec![m1, m2, m3].into_iter().collect(),
        )
    }
}
