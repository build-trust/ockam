/// Purpose for a [`super::purpose_key::PurposeKey`]
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Purpose {
    /// Purpose Key dedicated for Secure Channel creation
    SecureChannel,
    /// Purpose Key dedicated for Credentials issuing
    Credentials,
}
