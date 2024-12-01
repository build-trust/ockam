#![allow(missing_docs)]

use crate::messages::RoutingNumber;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

/// UDP Transport error type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UdpTransportError {
    InvalidTotalNumber(RoutingNumber),
    TotalNumberMismatch {
        routing_number: RoutingNumber,
        original_total_number: u16,
        actual_total_number: u16,
    },
    OutOfBounds {
        routing_number: RoutingNumber,
        total_number: u16,
        offset: u16,
    },
    DuplicateMessage {
        routing_number: RoutingNumber,
        offset: u16,
    },
    DuplicateLastMessage(RoutingNumber),
    UnexpectedLength {
        routing_number: RoutingNumber,
        expected_length: usize,
        actual_length: usize,
    },
    EmptyPayload(RoutingNumber),
    MessageLengthExceeded {
        routing_number: RoutingNumber,
        data_offset_end: usize,
    },
}

impl ockam_core::compat::error::Error for UdpTransportError {}
impl core::fmt::Display for UdpTransportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidTotalNumber(r) => write!(
                f,
                "Received invalid message with total number 0 for Routing number: {r}",
            ),
            Self::TotalNumberMismatch {
                routing_number,
                original_total_number,
                actual_total_number,
            } => {
                write!(f, "Received invalid message for Routing number: {routing_number}. Original total number: {original_total_number}, received total number: {actual_total_number}")
            }
            Self::OutOfBounds {
                routing_number,
                total_number,
                offset,
            } => {
                write!(
                    f,
                    "Received invalid message for Routing number: {routing_number}. Total: {total_number}, Offset: {offset}",
                )
            }
            Self::DuplicateMessage {
                routing_number,
                offset,
            } => {
                write!(
                    f,
                    "Received duplicate message for Routing number: {routing_number}. Offset: {offset}",
                )
            }
            Self::DuplicateLastMessage(routing_number) => {
                write!(
                    f,
                    "Received duplicate last message for Routing number: {routing_number}",
                )
            }
            Self::UnexpectedLength {
                routing_number,
                expected_length,
                actual_length,
            } => {
                write!(
                    f,
                    "Received invalid message for Routing number: {routing_number}. Expected len: {expected_length} Actual len: {actual_length}",
                )
            }
            Self::EmptyPayload(routing_number) => {
                write!(
                    f,
                    "Received invalid message with empty payload for Routing number: {routing_number}",
                )
            }
            Self::MessageLengthExceeded {
                routing_number,
                data_offset_end,
            } => {
                write!(
                    f,
                    "Message exceeded maximum limit. Routing number: {routing_number}, Data offset end: {data_offset_end}",
                )
            }
        }
    }
}

impl From<UdpTransportError> for Error {
    #[track_caller]
    fn from(err: UdpTransportError) -> Error {
        Error::new(Origin::Transport, Kind::Io, err)
    }
}
