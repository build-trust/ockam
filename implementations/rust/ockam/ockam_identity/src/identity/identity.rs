use crate::models::Identifier;
use crate::verified_change::VerifiedChange;
use ockam_core::Result;

#[derive(Clone, Debug)]
pub struct ChangeHistoryBinary(pub Vec<u8>);

/// Identity implementation
#[derive(Clone, Debug)]
pub struct Identity {
    identifier: Identifier,
    verified_changes: Vec<VerifiedChange>,
    // We preserve the original change_history binary
    // as serialization is not guaranteed to be deterministic
    change_history: ChangeHistoryBinary,
}

impl Identity {
    /// Create a new identity
    pub(crate) fn new(
        identifier: Identifier,
        verified_changes: Vec<VerifiedChange>,
        change_history: ChangeHistoryBinary,
    ) -> Self {
        Self {
            identifier,
            verified_changes,
            change_history,
        }
    }

    /// Return the identity identifier
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    pub fn verified_changes(&self) -> &[VerifiedChange] {
        self.verified_changes.as_slice()
    }

    /// `Identity` change history binary
    pub fn change_history(&self) -> &ChangeHistoryBinary {
        &self.change_history
    }
}

impl Identity {
    // /// Export an `Identity` to the binary format
    // pub fn export(&self) -> Vec<u8> {
    //     self.change_history.clone()
    // }
    //
    // /// Export an `Identity` as a hex-formatted string
    // pub fn export_hex(&self) -> Result<String> {
    //     Ok(hex::encode(&self.change_history_binary))
    // }

    // /// Create an Identity from serialized data
    // // FIXME: signatures verification should be mandatory
    // pub fn import(identifier: &IdentityIdentifier, data: &[u8]) -> Result<Identity> {
    //     let change_history = IdentityChangeHistory::import(data)?;
    //     Ok(Identity::new(identifier.clone(), change_history))
    // }
}

impl Identity {
    // /// Add a new key change to the change history
    // pub fn add_change(&mut self, change: IdentitySignedChange) -> Result<()> {
    //     self.change_history.add_change(change)
    // }

    // /// Return the root public key of an identity
    // pub fn get_root_public_key(&self) -> Result<PublicKey> {
    //     self.change_history.get_root_public_key()
    // }
    //
    // pub(crate) fn get_public_key(&self, key_label: Option<&str>) -> Result<PublicKey> {
    //     let key = match key_label {
    //         Some(label) => self.get_labelled_public_key(label)?,
    //         None => self.get_root_public_key()?,
    //     };
    //     Ok(key)
    // }
    //
    // pub(crate) fn get_labelled_public_key(&self, label: &str) -> Result<PublicKey> {
    //     self.change_history.get_public_key(label)
    // }
    //
    // /// Return the list of key changes for this identity
    // pub(crate) fn changes(&self) -> &IdentityChangeHistory {
    //     &self.change_history
    // }
    //
    // /// Compare to a previously known state of the same `Identity`
    // pub fn compare(&self, known: &Self) -> IdentityHistoryComparison {
    //     self.change_history.compare(&known.change_history)
    // }
}
