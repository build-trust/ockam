use crate::secure_channel::role::Role;
use ockam_core::Address;

// Previously there were regular ephemeral secure channel encryptor&decryptor
// and identity secure channel encryptor&decryptor.
// Now this logic is merged into one encryptor&decryptor pair, but for backwards
// compatibility each of them have more addresses to simulate old behaviour.
#[derive(Clone, Debug)]
pub(crate) struct Addresses {
    // Used to send decrypted messages and secure channel creation completion notification
    pub(crate) decryptor_internal: Address,
    // Used for KeyExchange and receiving encrypted messages
    pub(crate) decryptor_remote: Address,
    // Used to encrypt messages without sending them with Ockam Routing to the other end of the channel
    pub(crate) decryptor_api: Address,

    // Encryptor worker address used to receive plain messages that will be encrypted and forwarded
    // to the other end of the channel
    pub(crate) encryptor: Address,
    // Used to decrypt messages that were received though some channel other than Ockam Routing from the other end of the channel
    pub(crate) encryptor_api: Address,
}

impl Addresses {
    pub(crate) fn generate(role: Role) -> Self {
        let role_str = role.str();
        let decryptor_internal =
            Address::random_tagged(&format!("SecureChannel.{}.decryptor.internal", role_str));
        let decryptor_remote =
            Address::random_tagged(&format!("SecureChannel.{}.decryptor.remote", role_str));
        let decryptor_api =
            Address::random_tagged(&format!("SecureChannel.{}.decryptor.api", role_str));

        let encryptor = Address::random_tagged(&format!("SecureChannel.{}.encryptor", role_str));
        let encryptor_api =
            Address::random_tagged(&format!("SecureChannel.{}.encryptor.api", role_str));

        Self {
            decryptor_internal,
            decryptor_remote,
            decryptor_api,
            encryptor,
            encryptor_api,
        }
    }
}
