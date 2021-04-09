use blake2::VarBlake2b;
use bls12_381_plus::{G1Projective, Scalar};
use digest::{Update, VariableOutput};
use serde::{
    de::{Error as DError, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};
use short_group_signatures_core::lib::*;
use subtle::CtOption;

macro_rules! slicer {
    ($d:expr, $b:expr, $e:expr, $s:expr) => {
        &<[u8; $s]>::try_from(&$d[$b..$e]).unwrap();
    };
}

pub(crate) trait VecSerializer<'de>: Sized {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

impl<'de, T, L> VecSerializer<'de> for Vec<T, L>
where
    T: Default + Serialize + Deserialize<'de>,
    L: ArrayLength<T>,
{
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.len() {
            0 => {
                let seq = s.serialize_seq(None)?;
                seq.end()
            }
            l => {
                let mut seq = s.serialize_seq(Some(l))?;
                for i in 0..l {
                    seq.serialize_element(&self[i])?;
                }
                seq.end()
            }
        }
    }

    fn deserialize<D>(d: D) -> Result<Vec<T, L>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VecVisitor<T, L> {
            element: PhantomData<T>,
            length: PhantomData<L>,
        }

        impl<'de, T, L> Visitor<'de> for VecVisitor<T, L>
        where
            T: Default + Deserialize<'de>,
            L: ArrayLength<T>,
        {
            type Value = Vec<T, L>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "an array of {}", stringify!(T))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut arr = Vec::new();
                while let Some(c) = seq.next_element()? {
                    arr.push(c)
                        .map_err(|_| A::Error::invalid_length(arr.len(), &self))?;
                }
                Ok(arr)
            }
        }

        let visitor = VecVisitor {
            element: PhantomData,
            length: PhantomData,
        };
        d.deserialize_seq(visitor)
    }
}

pub fn hash_to_scalar<B: AsRef<[u8]>>(data: B) -> Scalar {
    const BYTES: usize = 48;
    let mut res = [0u8; BYTES];
    let mut hasher = VarBlake2b::new(BYTES).unwrap();
    hasher.update(data.as_ref());
    hasher.finalize_variable(|out| {
        res.copy_from_slice(out);
    });
    Scalar::from_okm(&res)
}

pub fn scalar_to_bytes(s: Scalar) -> [u8; 32] {
    let mut bytes = s.to_bytes();
    // Make big endian
    bytes.reverse();
    bytes
}

pub fn scalar_from_bytes(bytes: &[u8; 32]) -> CtOption<Scalar> {
    let mut t = [0u8; 32];
    t.copy_from_slice(bytes);
    t.reverse();
    Scalar::from_bytes(&t)
}

pub fn sum_of_products(points: &[G1Projective], scalars: &mut [Scalar]) -> G1Projective {
    G1Projective::sum_of_products_in_place(points, scalars)
}

#[cfg(test)]
pub struct MockRng(rand_xorshift::XorShiftRng);

#[cfg(test)]
impl rand_core::SeedableRng for MockRng {
    type Seed = [u8; 16];

    fn from_seed(seed: Self::Seed) -> Self {
        Self(rand_xorshift::XorShiftRng::from_seed(seed))
    }
}

#[cfg(test)]
impl rand_core::CryptoRng for MockRng {}

#[cfg(test)]
impl rand_core::RngCore for MockRng {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.0.try_fill_bytes(dest)
    }
}
