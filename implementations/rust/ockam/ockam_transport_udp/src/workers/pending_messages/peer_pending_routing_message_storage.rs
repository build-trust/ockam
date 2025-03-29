use crate::messages::{RoutingNumber, UdpRoutingMessage, UdpTransportMessage};
use crate::workers::pending_messages::{PendingMessage, PendingMessageState};
use core::cmp::min;
use ockam_core::compat::collections::VecDeque;
use ockam_core::Result;
use tracing::{error, trace};

/// Pending routing messages for a certain peer
/// This storage will cache packets (until they fit into the cache) and assemble them into
/// full Ockam Routing message when all parts are receiver. Then the Ockam routing message is
/// sent to the node.
pub(crate) struct PeerPendingRoutingMessageStorage {
    // Reusable buffers to avoid excess allocations
    // TODO: Can we share them between other peers?
    buffer_queue: VecDeque<Vec<u8>>,
    // Oldest routing number we can accept
    oldest_routing_number: RoutingNumber,
    // Max number of messages we cache
    max_pending_messages: u16,
    // Messages with following routing numbers:
    // [self.oldest_routing_number, ..., self.oldest_routing_number + max_pending_messages - 1]
    pending_messages: Vec<PendingMessageState>,
}

impl PeerPendingRoutingMessageStorage {
    // Create given the first received message
    pub(crate) fn new(routing_number: RoutingNumber, max_pending_messages: u16) -> Self {
        let mut pending_messages = Vec::with_capacity(max_pending_messages as usize);
        pending_messages.resize_with(max_pending_messages as usize, Default::default);

        Self {
            buffer_queue: Default::default(),
            oldest_routing_number: routing_number,
            max_pending_messages,
            pending_messages,
        }
    }

    pub(crate) fn add_transport_message_and_try_assemble(
        &mut self,
        transport_message: UdpTransportMessage<'_>,
    ) -> Result<Option<UdpRoutingMessage<'static>>> {
        trace!(
            "Received routing message {}, offset {}",
            transport_message.routing_number,
            transport_message.offset
        );

        // self.oldest_routing_number is the oldest message we can accept,
        // older than that are ignored
        if transport_message.routing_number < self.oldest_routing_number {
            trace!(
                "Dropping routing message: {} because it arrived late. Offset {}",
                transport_message.routing_number,
                transport_message.offset
            );

            return Ok(None);
        }

        // We received a newer message
        let diff = transport_message.routing_number - self.oldest_routing_number;

        // Move self.pending_messages if needed and update the diff
        let diff = if diff >= self.max_pending_messages {
            // We received a much newer message, we need to drop one or few older messages so that
            // this message fits into our self.pending_messages

            // Length of the shift we need to perform on our self.pending_messages array
            let shift = diff - self.max_pending_messages + 1;

            // Drop the messages that don't fit anymore
            let number_of_messages_to_drop = min(shift, self.max_pending_messages) as usize;

            for i in 0..number_of_messages_to_drop {
                match self.pending_messages[i].take() {
                    PendingMessageState::NotReceived => {
                        trace!(
                            "Discarding old not received routing message {} because a new routing message has arrived: {}",
                            self.oldest_routing_number + (i as u16),
                            transport_message.routing_number
                        );
                    }
                    PendingMessageState::InProgress(pending_message) => {
                        trace!(
                            "Discarding old partially received routing message {} because a new routing message has arrived: {}",
                            self.oldest_routing_number + (i as u16),
                            transport_message.routing_number
                        );

                        // Put the buffer back to reuse in the future
                        let buffer = pending_message.drop_message();
                        self.buffer_queue.push_back(buffer);
                    }
                    PendingMessageState::FullyHandled => {}
                }
            }

            // If we didn't drop all the messages, move the rest to the left
            if shift < self.max_pending_messages {
                let number_of_messages_to_shift = (self.max_pending_messages - shift) as usize;
                for i in 0..number_of_messages_to_shift {
                    self.pending_messages[i] = self.pending_messages[i + shift as usize].take();
                }
            }

            self.oldest_routing_number += shift;

            (diff - shift) as usize
        } else {
            diff as usize
        };

        let pending_message_state = self.pending_messages[diff].take();

        let mut pending_message = match pending_message_state {
            PendingMessageState::NotReceived => {
                let buffer = self.buffer_queue.pop_front().unwrap_or_default();

                PendingMessage::new(buffer)
            }
            PendingMessageState::InProgress(m) => m,
            PendingMessageState::FullyHandled => {
                // Already send out.
                return Ok(None);
            }
        };

        pending_message.add_transport_message(transport_message)?;

        match pending_message.try_assemble() {
            Some(routing_message_binary) => {
                let res = match minicbor::decode::<UdpRoutingMessage>(&routing_message_binary) {
                    Ok(routing_message) => Some(routing_message.into_owned()),
                    Err(err) => {
                        error!("Error while decoding UDP message {}", err);
                        None
                    }
                };

                self.pending_messages[diff] = PendingMessageState::FullyHandled;

                Ok(res)
            }
            None => {
                self.pending_messages[diff] = PendingMessageState::InProgress(pending_message);
                Ok(None)
            }
        }
    }
}
