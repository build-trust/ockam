use crate::messages::{RoutingNumber, UdpTransportMessage};
use crate::{UdpTransportError, MAX_MESSAGE_SIZE};
use ockam_core::compat::collections::HashSet;
use ockam_core::Result;
use std::mem;
use tracing::trace;

pub(crate) struct PendingMessage {
    // 0 means uninitialized for these 3 values
    routing_number: RoutingNumber,
    total: u16,
    // We expect all packets to be the same length except the first one
    packet_payload_length: usize,

    // TODO: I think usually total number is low, so we can use bitmap or vec here
    not_received_parts: HashSet<u16>,
    // Whole assembled message
    binary: Vec<u8>,
    // Last part is treated differently, because it can be arbitrary length
    last_part: Option<Vec<u8>>,
}

impl PendingMessage {
    pub(crate) fn new(binary: Vec<u8>) -> Self {
        Self {
            routing_number: RoutingNumber(0),
            total: 0,
            packet_payload_length: 0,
            not_received_parts: Default::default(),
            binary,
            last_part: None,
        }
    }

    fn initialize_fields_if_needed(&mut self, transport_message: &UdpTransportMessage<'_>) {
        if self.total == 0 {
            self.total = transport_message.total;
            self.routing_number = transport_message.routing_number;

            for i in 0..self.total {
                self.not_received_parts.insert(i);
            }
        }

        // Last message may have a different length
        if self.packet_payload_length == 0 && !self.is_last_message(transport_message) {
            self.packet_payload_length = transport_message.payload.len();
        }
    }

    fn merge_last_part_if_needed(&mut self) -> Result<()> {
        let last_part = if let Some(last_part) = self.last_part.take() {
            last_part
        } else {
            return Ok(());
        };

        let data_offset_begin = self.binary.len();
        let data_offset_end = data_offset_begin + last_part.len();

        if data_offset_end > MAX_MESSAGE_SIZE {
            return Err(UdpTransportError::MessageLengthExceeded {
                routing_number: self.routing_number,
                data_offset_end,
            })?;
        }

        self.binary.resize(data_offset_end, 0);
        self.binary[data_offset_begin..data_offset_end].copy_from_slice(&last_part);

        Ok(())
    }

    fn is_last_message(&self, transport_message: &UdpTransportMessage<'_>) -> bool {
        transport_message.offset + 1 == self.total
    }

    pub(crate) fn add_transport_message(
        &mut self,
        transport_message: UdpTransportMessage<'_>,
    ) -> Result<()> {
        if transport_message.total == 0 {
            return Err(UdpTransportError::InvalidTotalNumber(
                transport_message.routing_number,
            ))?;
        }

        if transport_message.payload.is_empty() {
            return Err(UdpTransportError::EmptyPayload(
                transport_message.routing_number,
            ))?;
        }

        self.initialize_fields_if_needed(&transport_message);

        if self.total != transport_message.total {
            return Err(UdpTransportError::TotalNumberMismatch {
                routing_number: self.routing_number,
                original_total_number: self.total,
                actual_total_number: transport_message.total,
            })?;
        }

        if self.total <= transport_message.offset {
            return Err(UdpTransportError::OutOfBounds {
                routing_number: self.routing_number,
                total_number: self.total,
                offset: transport_message.offset,
            })?;
        }

        if self.is_last_message(&transport_message) {
            if self.last_part.is_some() {
                return Err(UdpTransportError::DuplicateLastMessage(self.routing_number))?;
            }
        } else if self.packet_payload_length != transport_message.payload.len() {
            return Err(UdpTransportError::UnexpectedLength {
                routing_number: self.routing_number,
                expected_length: self.packet_payload_length,
                actual_length: transport_message.payload.len(),
            })?;
        }

        if !self.not_received_parts.remove(&transport_message.offset) {
            return Err(UdpTransportError::DuplicateMessage {
                routing_number: self.routing_number,
                offset: transport_message.offset,
            })?;
        }

        if self.is_last_message(&transport_message) {
            self.last_part = Some(transport_message.payload.into_owned());
        } else {
            let data_offset_begin =
                transport_message.offset as usize * transport_message.payload.len();
            let data_offset_end = data_offset_begin + transport_message.payload.len();

            if data_offset_end > MAX_MESSAGE_SIZE {
                return Err(UdpTransportError::MessageLengthExceeded {
                    routing_number: self.routing_number,
                    data_offset_end,
                })?;
            }

            if self.binary.len() < data_offset_end {
                self.binary.resize(data_offset_end, 0);
            }

            trace!(
                "Filling message data. Routing number: {}. Offset: {}. Data offset: {}..{}",
                self.routing_number,
                transport_message.offset,
                data_offset_begin,
                data_offset_end
            );
            self.binary[data_offset_begin..data_offset_end]
                .copy_from_slice(&transport_message.payload);
        }

        if self.not_received_parts.is_empty() {
            self.merge_last_part_if_needed()?;
        }

        Ok(())
    }

    pub(crate) fn try_assemble(&mut self) -> Option<Vec<u8>> {
        if self.not_received_parts.is_empty() && self.total != 0 {
            // We will return the clone here, but we'll reuse the original buffer later
            Some(self.binary.clone())
        } else {
            None
        }
    }

    pub(crate) fn drop_message(mut self) -> Vec<u8> {
        self.binary.clear();
        self.binary
    }
}

pub(crate) enum PendingMessageState {
    NotReceived,
    InProgress(PendingMessage),
    FullyHandled,
}

impl Default for PendingMessageState {
    fn default() -> Self {
        Self::NotReceived
    }
}

impl PendingMessageState {
    pub(crate) fn take(&mut self) -> Self {
        mem::replace(self, Self::NotReceived)
    }
}
