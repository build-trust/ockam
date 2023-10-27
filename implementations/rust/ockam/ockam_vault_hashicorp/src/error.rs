use ockam_core::errcode::{Kind, Origin};
use thiserror::Error;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("key was not found")]
    KeyNotFound,
}

impl From<Error> for ockam_core::Error {
    fn from(e: Error) -> Self {
        ockam_core::Error::new(Origin::Other, Kind::Io, e)
    }
}
