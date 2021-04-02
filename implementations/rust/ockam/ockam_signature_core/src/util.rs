use crate::lib::*;
use heapless::ArrayLength;
use serde::{
    de::{Error as DError, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};

/// An internal vector serializer
pub trait VecSerializer<'de>: Sized {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
    fn deserialize<D>(des: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

impl<'de, T, N> VecSerializer<'de> for Vec<T, N>
where
    T: Default + Copy + Serialize + Deserialize<'de>,
    N: ArrayLength<T>,
{
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let l = if self.is_empty() {
            None
        } else {
            Some(self.len())
        };
        let mut iter = ser.serialize_seq(l)?;
        for i in self {
            iter.serialize_element(i)?;
        }
        iter.end()
    }

    fn deserialize<D>(des: D) -> Result<Vec<T, N>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TVisitor<T, N> {
            element: PhantomData<T>,
            size: PhantomData<N>,
        }

        impl<'de, T, N> Visitor<'de> for TVisitor<T, N>
        where
            T: Default + Copy + Deserialize<'de>,
            N: ArrayLength<T>,
        {
            type Value = Vec<T, N>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("expected array")
            }

            fn visit_seq<A>(self, mut arr: A) -> Result<Vec<T, N>, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut buf = Vec::new();
                for i in 0..N::to_usize() {
                    buf.push(
                        arr.next_element()?
                            .ok_or_else(|| DError::invalid_length(i, &self))?,
                    )
                    .map_err(|_| DError::invalid_length(i, &self))?;
                }
                Ok(buf)
            }
        }

        let visitor = TVisitor {
            element: PhantomData,
            size: PhantomData,
        };
        des.deserialize_seq(visitor)
    }
}
