use core::fmt::{Debug, Formatter};
use minicbor::{CborLen, Decode, Encode};
use rand::random;
use std::cmp::Ordering;
use std::ops::{Add, AddAssign, Sub};

/// Number of the [`UdpRoutingMessage`]. Each [`UdpRoutingMessage`] is assigned a value, which
/// helps the receiver to assemble the message. Start with a random value and uses overflowing
/// arithmetics for increment.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Encode, Decode, CborLen)]
#[cbor(transparent)]
pub struct RoutingNumber(#[n(0)] pub u16);

impl core::fmt::Display for RoutingNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.0, f)
    }
}

impl Default for RoutingNumber {
    fn default() -> Self {
        Self(random())
    }
}

impl RoutingNumber {
    pub fn new(value: u16) -> Self {
        Self(value)
    }

    pub fn increment(&mut self) {
        *self += 1;
    }
}

impl PartialOrd for RoutingNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RoutingNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.0 == other.0 {
            return Ordering::Equal;
        }

        let (self_opposite, overflow) = self.0.overflowing_add(u16::MAX / 2 + 1);

        #[allow(clippy::collapsible_else_if)]
        if overflow {
            if self.0 > other.0 && other.0 > self_opposite {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        } else {
            if self.0 < other.0 && other.0 <= self_opposite {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
    }
}

impl AddAssign<u16> for RoutingNumber {
    fn add_assign(&mut self, rhs: u16) {
        self.0 = self.0.wrapping_add(rhs);
    }
}

impl Add<u16> for RoutingNumber {
    type Output = u16;

    fn add(self, rhs: u16) -> Self::Output {
        let mut s = self;
        s += rhs;
        s.0
    }
}

impl Sub<Self> for RoutingNumber {
    type Output = u16;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0.wrapping_sub(rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::messages::RoutingNumber;

    #[test]
    fn default_is_random() {
        assert_ne!(RoutingNumber::default(), RoutingNumber::default());
    }

    #[test]
    fn increment() {
        let mut number = RoutingNumber::default();
        let n1 = number.0;
        number.increment();
        assert_eq!(number.0, n1 + 1);
    }

    #[test]
    fn increment_overflow() {
        let mut number = RoutingNumber(u16::MAX);
        number.increment();
        assert_eq!(number.0, 0);
    }

    #[test]
    fn comparison() {
        assert!(RoutingNumber(5) < RoutingNumber(6));
        assert!(RoutingNumber(6) > RoutingNumber(5));
        assert_eq!(RoutingNumber(5), RoutingNumber(5));
    }

    #[test]
    fn comparison_overflow() {
        assert!(RoutingNumber(u16::MAX) < RoutingNumber(0));
        assert!(RoutingNumber(u16::MAX) > RoutingNumber(u16::MAX - 1));
        assert!(RoutingNumber(0) > RoutingNumber(u16::MAX));
    }
}
