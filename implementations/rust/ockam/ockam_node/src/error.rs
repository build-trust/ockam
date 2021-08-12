use crate::tokio::{self, sync::mpsc::error::SendError};
use core::fmt::Debug;

/// Error declarations.
#[derive(Clone, Copy, Debug)]
pub enum Error {
    /// No error
    None,
    /// Unable to gracefully stop the Node.
    FailedStopNode,
    /// Unable to start a worker
    FailedStartWorker,
    /// Worker start failed because the address was already taken
    WorkerAddressTaken,
    /// The requested worker address is unknown
    UnknownWorker,
    /// Unable to stop a worker
    FailedStopWorker,
    /// Unable to list available workers
    FailedListWorker,
    /// Unable to send a message to a worker
    FailedSendMessage,
    /// Unable to receive the desired piece of data
    FailedLoadData,
    /// An umbrella for internal I/O failures
    InternalIOFailure,
    /// Worker tried to send message from foreign address
    SenderAddressDoesntExist,
    /// Operation timed out
    Timeout,
}

impl Error {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 11_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM_NODE";
}

impl From<Error> for ockam_core::Error {
    fn from(e: Error) -> ockam_core::Error {
        ockam_core::Error::new(Error::DOMAIN_CODE + (e as u32), Error::DOMAIN_NAME)
    }
}

impl From<crate::NodeError> for ockam_core::Error {
    fn from(err: crate::NodeError) -> Self {
        use crate::NodeError::*;
        match err {
            NoSuchWorker(_) => Error::UnknownWorker,
            WorkerExists(_) => Error::WorkerAddressTaken,
            RouterExists => Error::InternalIOFailure,
        }
        .into()
    }
}

impl<T: Debug> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Error {
        Error::InternalIOFailure
    }
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        Self::Timeout
    }
}
