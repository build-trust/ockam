// This is copied verbatim(Some stuff we don't need removed) from [embassy](https://github.com/embassy-rs/embassy/blob/7561fa19348530ce85e2645e0be8801b9b2bbe13/embassy-net/src/packet_pool.rs)
// One advantage of using atomic_pool instead of heapless it doesn't seem to have the same kind of ABA problem.
use core::ops::{Deref, DerefMut, Range};

use atomic_pool::{pool, Box};

const MTU: usize = 1516;

#[cfg(feature = "pool-4")]
pub(crate) const PACKET_POOL_SIZE: usize = 4;

#[cfg(feature = "pool-8")]
pub(crate) const PACKET_POOL_SIZE: usize = 8;

#[cfg(feature = "pool-16")]
pub(crate) const PACKET_POOL_SIZE: usize = 16;

#[cfg(feature = "pool-32")]
pub(crate) const PACKET_POOL_SIZE: usize = 32;

pool!(pub(crate) PacketPool: [Packet; PACKET_POOL_SIZE]);
pub(crate) type PacketBox = Box<PacketPool>;

#[repr(align(4))]
pub(crate) struct Packet([u8; MTU]);

impl Packet {
    pub(crate) const fn new() -> Self {
        Self([0; MTU])
    }
}

pub(crate) trait PacketBoxExt {
    fn slice(self, range: Range<usize>) -> PacketBuf;
}

impl PacketBoxExt for PacketBox {
    fn slice(self, range: Range<usize>) -> PacketBuf {
        PacketBuf {
            packet: self,
            range,
        }
    }
}

impl Deref for Packet {
    type Target = [u8; MTU];

    fn deref(&self) -> &[u8; MTU] {
        &self.0
    }
}

impl DerefMut for Packet {
    fn deref_mut(&mut self) -> &mut [u8; MTU] {
        &mut self.0
    }
}

pub struct PacketBuf {
    packet: PacketBox,
    range: Range<usize>,
}

impl Deref for PacketBuf {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.packet[self.range.clone()]
    }
}

impl DerefMut for PacketBuf {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.packet[self.range.clone()]
    }
}
