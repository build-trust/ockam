use crate::tokio::{self, sync::mpsc::error::SendError};
use core::fmt::Debug;

/// Error declarations.
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug)]
pub enum Error {
    /// Unable to gracefully stop the Node.
    FailedStopNode = 1,
    /// Unable to start a worker
    FailedStartWorker,
    /// Unable to start a processor
    FailedStartProcessor,
    /// Worker start failed because the address was already taken
    WorkerAddressTaken,
    /// The requested worker address is unknown
    UnknownWorker,
    /// The requested processor address is unknown
    UnknownProcessor,
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
    /// Error while receiving shutdown acknowledgment
    ShutdownAckError,
    /// Error while receiving shutdown signal
    ShutdownRxError,
    /// Operation timed out
    Timeout,
    /// Executor body join error
    ExecutorBodyJoinError,
    /// Given command was rejected
    CommandRejected,
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
            NoSuchProcessor(_) => Error::UnknownProcessor,
            WorkerExists(_) => Error::WorkerAddressTaken,
            RouterExists => Error::InternalIOFailure,
            Rejected(_) => Error::CommandRejected,
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
