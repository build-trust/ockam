use core::cmp::Ordering;

use crate::driver::BleEvent;
use crate::driver::CHARACTERISTIC_VALUE_LENGTH;
use crate::driver::MAX_OCKAM_MESSAGE_LENGTH;
use crate::error::BleError;

/// PacketBuffer
pub struct PacketBuffer {
    fragment_len: usize,
    buffer: [u8; MAX_OCKAM_MESSAGE_LENGTH],
    packet_len: usize,
    offset: usize,
}

impl Default for PacketBuffer {
    fn default() -> Self {
        Self {
            fragment_len: CHARACTERISTIC_VALUE_LENGTH,
            buffer: [0_u8; MAX_OCKAM_MESSAGE_LENGTH],
            packet_len: 0,
            offset: 0,
        }
    }
}

impl PacketBuffer {
    pub fn packet_len(&self) -> usize {
        self.packet_len
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// PacketBuffer send implementation
impl PacketBuffer {
    pub fn from_packet(packet: &[u8]) -> PacketBuffer {
        if packet.len() > MAX_OCKAM_MESSAGE_LENGTH {
            error!(
                "Packet too long for PacketBuffer: {} > {} ",
                packet.len(),
                MAX_OCKAM_MESSAGE_LENGTH
            );
            panic!(
                "Packet too long for PacketBuffer: {} > {} ",
                packet.len(),
                MAX_OCKAM_MESSAGE_LENGTH
            );
        }

        let mut pb = PacketBuffer {
            packet_len: packet.len(),
            ..Default::default()
        };

        let destination = &mut pb.buffer[..pb.packet_len];
        destination.copy_from_slice(&packet[..pb.packet_len]);

        pb
    }

    pub fn send_packet_length(&mut self) -> [u8; 8] {
        let bytes: [u8; 8] = (self.packet_len as u64).to_be_bytes();
        bytes
    }

    pub fn send_next_fragment(&mut self) -> Option<&[u8]> {
        let bytes_left: isize = self.packet_len as isize - self.offset as isize;
        match bytes_left.cmp(&0) {
            Ordering::Less => panic!("Packet buffer has no more packets left to give"),
            Ordering::Greater => {
                // copy packet data to fragment
                let fragment_len = core::cmp::min(self.fragment_len, bytes_left as usize);
                let end = self.offset + fragment_len;
                let fragment = &self.buffer[self.offset..end];
                self.offset += fragment_len;
                Some(fragment)
            }
            Ordering::Equal => {
                trace!("Received complete packet");
                None
            }
        }
    }
}

/// PacketBuffer receive implementation
impl PacketBuffer {
    pub fn receive_packet_length(&mut self, fragment: &[u8]) -> Option<usize> {
        if fragment.len() != 8 {
            return None;
        }

        let mut buffer = [0_u8; 8];
        buffer.copy_from_slice(&fragment[..8]);
        let packet_len = u64::from_be_bytes(buffer);
        if packet_len > self.buffer.len() as u64 {
            error!("Packet is too long for packet buffer: {}", packet_len);
            return None;
        }

        // TODO Currently we only support packet lenghts up to 2^16
        // bits long.
        //
        // The reason for this is that I'm too lazy right now to setup
        // a control channel for this BLE UART implementation and
        // instead rely on the probability of receiving an 8 byte long
        // packet consisting of 6 zero bytes followed by two bytes
        // that can be non-zero.
        if packet_len > (2 << 16) {
            error!(
                "Packet is too long for this implementation.\n \
                    See `ockam_transport_ble/src/packet.rs` for more \
                    information: {}",
                fragment.len()
            );
            return None;
        }

        trace!("Received packet length: {} bytes", packet_len);

        self.reset();
        self.packet_len = packet_len as usize;

        Some(self.packet_len)
    }

    pub fn receive_next_fragment(&mut self, fragment: &[u8]) -> ockam::Result<Option<&[u8]>> {
        if self.offset >= self.packet_len {
            panic!("packet buffer already has enough fragments")
        }

        let fragment_len = fragment.len();
        if self.offset + fragment_len > self.buffer.len() {
            error!(
                "Received packet fragment is too long for packet buffer: {}",
                self.buffer.len()
            );
            return Err(BleError::ReadError.into());
        }

        // append fragment to packet buffer
        self.buffer[self.offset..self.offset + fragment_len]
            .copy_from_slice(&fragment[..fragment_len]);
        self.offset += fragment_len;

        trace!(
            "PacketBuffer::receive_next_fragment received: {} of {}",
            self.offset,
            self.packet_len
        );

        // check if we have a complete packet
        let bytes_left: isize = self.packet_len as isize - self.offset as isize;
        if bytes_left <= 0 {
            trace!(
                "PacketBuffer::receive_next_fragment has a complete packet, bytes_left: {}",
                bytes_left
            );
            let packet = &self.buffer[..self.packet_len];
            return Ok(Some(packet));
        }

        Ok(None)
    }
}

impl core::fmt::Debug for PacketBuffer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let packet = &self.buffer[..self.packet_len];
        write!(f, "PacketBuffer[..{}] = {:?}", self.offset, packet)
    }
}
