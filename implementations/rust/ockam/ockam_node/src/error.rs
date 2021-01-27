use ockam_core::Error;

#[derive(Clone, Copy, Debug)]
pub enum NodeError {
    None,
    CouldNotStop,
}

impl NodeError {
    pub const DOMAIN_CODE: u32 = 11_000;
    pub const DOMAIN_NAME: &'static str = "OCKAM_NODE";
}

impl Into<Error> for NodeError {
    fn into(self) -> Error {
        Error::new(Self::DOMAIN_CODE + (self as u32), Self::DOMAIN_NAME)
    }
}
