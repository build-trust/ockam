use core::fmt::{Debug, Display, Formatter};
use core::hash::{Hash, Hasher};
use core::ops::Deref;
use minicbor::encode::{Error, Write};
use minicbor::{CborLen, Decode, Encode, Encoder};
use std::ops::DerefMut;
use zeroize::Zeroize;

/// OnDrop is an enum to specify whether to zeroize the inner value when dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub enum OnDrop {
    /// Do not zeroize the inner value when dropped
    #[n(0)] NoZeroize,
    /// Zeroize the inner value when dropped
    #[n(1)] Zeroize,
}

/// MaybeZeroizeOnDrop will zeroize the inner value when dropped when zeroize_on_drop is true.
pub struct MaybeZeroizeOnDrop<T: Zeroize> {
    target: T,
    on_drop: OnDrop,
}

impl<T: Zeroize + Default> MaybeZeroizeOnDrop<T> {
    /// Create a new MaybeZeroizeOnDrop with the given target and zeroize_on_drop flag.
    pub fn new(target: T, on_drop: OnDrop) -> Self {
        Self { target, on_drop }
    }

    /// Gets on_drop
    pub fn on_drop(&self) -> OnDrop {
        self.on_drop
    }

    /// Sets on_drop
    pub fn set_zeroize(&mut self, on_drop: OnDrop) {
        self.on_drop = on_drop;
    }
}

impl<T: Zeroize + Default> MaybeZeroizeOnDrop<T> {
    /// Return the inner value regardless of the zeroize_on_drop flag.
    /// The caller has the responsibility to ensure that the inner value is zeroized when necessary.
    pub fn discard_zeroize(mut self) -> T {
        std::mem::take(&mut self.target)
    }
}

impl<T: Zeroize> Deref for MaybeZeroizeOnDrop<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.target
    }
}

impl<T: Zeroize> DerefMut for MaybeZeroizeOnDrop<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.target
    }
}

impl<T: Zeroize> Drop for MaybeZeroizeOnDrop<T> {
    fn drop(&mut self) {
        if let OnDrop::Zeroize = self.on_drop {
            self.target.zeroize();
        }
    }
}

impl<T: Zeroize> Zeroize for MaybeZeroizeOnDrop<T> {
    fn zeroize(&mut self) {
        self.target.zeroize();
    }
}

impl<T: Zeroize + Default> Default for MaybeZeroizeOnDrop<T> {
    fn default() -> Self {
        Self {
            target: T::default(),
            on_drop: OnDrop::NoZeroize,
        }
    }
}

impl<T: Zeroize + Debug> Debug for MaybeZeroizeOnDrop<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaybeZeroizeOnDrop")
            .field("target", &self.target)
            .field("on_drop", &self.on_drop)
            .finish()
    }
}

impl<T: Zeroize + Display> Display for MaybeZeroizeOnDrop<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.target.fmt(f)
    }
}

impl<T: Zeroize + Clone> Clone for MaybeZeroizeOnDrop<T> {
    fn clone(&self) -> Self {
        Self {
            target: self.target.clone(),
            on_drop: self.on_drop,
        }
    }
}

impl<T: Zeroize + PartialEq> PartialEq for MaybeZeroizeOnDrop<T> {
    fn eq(&self, other: &Self) -> bool {
        self.target.eq(&other.target)
    }
}

impl<T: Zeroize + Eq> Eq for MaybeZeroizeOnDrop<T> {}

impl<T: Zeroize + PartialOrd> PartialOrd for MaybeZeroizeOnDrop<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.target.partial_cmp(&other.target)
    }
}

impl<T: Zeroize + Ord> Ord for MaybeZeroizeOnDrop<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.target.cmp(&other.target)
    }
}

impl<T: Zeroize + Hash> Hash for MaybeZeroizeOnDrop<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.target.hash(state)
    }
}

// serde serialize/deserialize impls
impl<T: Zeroize + serde::Serialize> serde::Serialize for MaybeZeroizeOnDrop<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.target.serialize(serializer)
    }
}

impl<'de, T: Zeroize + serde::Deserialize<'de>> serde::Deserialize<'de> for MaybeZeroizeOnDrop<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(|target| Self {
            target,
            on_drop: OnDrop::NoZeroize,
        })
    }
}

impl<C, T: Zeroize + Encode<C>> Encode<C> for MaybeZeroizeOnDrop<T> {
    fn encode<W: Write>(&self, e: &mut Encoder<W>, ctx: &mut C) -> Result<(), Error<W::Error>> {
        self.target.encode(e, ctx)
    }

    fn is_nil(&self) -> bool {
        self.target.is_nil()
    }
}

impl<'b, C, T: Zeroize + Decode<'b, C>> Decode<'b, C> for MaybeZeroizeOnDrop<T> {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        T::decode(d, ctx).map(|target| Self {
            target,
            on_drop: OnDrop::NoZeroize,
        })
    }
}

impl<C, T: Zeroize + CborLen<C>> CborLen<C> for MaybeZeroizeOnDrop<T> {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        self.target.cbor_len(ctx)
    }
}
