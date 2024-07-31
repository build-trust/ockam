/// Purpose for a [`PurposeKeys`](crate::PurposeKeys)
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Purpose {
    /// Purpose Key dedicated for Secure Channel creation
    SecureChannel,
    /// Purpose Key dedicated for Credentials issuing
    Credentials,
}
