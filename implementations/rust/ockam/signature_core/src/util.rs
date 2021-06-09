use crate::lib::*;
use blake2::VarBlake2b;
use bls12_381_plus::{G1Projective, Scalar};
use digest::{Update, VariableOutput};
use serde::{
    de::{Error as DError, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};
use subtle::CtOption;

/// Convert slice to a fixed array
#[macro_export]
macro_rules! slicer {
    ($d:expr, $b:expr, $e:expr, $s:expr) => {
        &<[u8; $s]>::try_from(&$d[$b..$e]).unwrap()
    };
}

/// An internal vector serializer
pub trait VecSerializer<'de>: Sized {
    /// Serialize the custom type and size
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
    /// Deserialize the custom type and size
    fn deserialize<D>(des: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

impl<'de, T, const N: usize> VecSerializer<'de> for Vec<T, N>
where
    T: Default + Copy + Serialize + Deserialize<'de>,
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
        struct TVisitor<T, const N: usize> {
            element: PhantomData<T>,
        }

        impl<'de, T, const N: usize> Visitor<'de> for TVisitor<T, N>
        where
            T: Default + Copy + Deserialize<'de>,
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
                for i in 0..N {
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
        };
        des.deserialize_seq(visitor)
    }
}

/// Hashes a byte sequence to a Scalar
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

/// Converts a scalar to big endian bytes
pub fn scalar_to_bytes(s: Scalar) -> [u8; 32] {
    let mut bytes = s.to_bytes();
    // Make big endian
    bytes.reverse();
    bytes
}

/// Convert a big endian byte sequence to a Scalar
pub fn scalar_from_bytes(bytes: &[u8; 32]) -> CtOption<Scalar> {
    let mut t = [0u8; 32];
    t.copy_from_slice(bytes);
    t.reverse();
    Scalar::from_bytes(&t)
}

/// Compute multi-exponeniation which for elliptic curves is the sum of products
/// using Pippenger's method
pub fn sum_of_products(points: &[G1Projective], scalars: &mut [Scalar]) -> G1Projective {
    G1Projective::sum_of_products_in_place(points, scalars)
}

#[cfg(test)]
mod test {
    use crate::lib::Vec;
    use crate::lib::*;
    use crate::util::{hash_to_scalar, scalar_from_bytes, scalar_to_bytes};
    use bls12_381_plus::{G1Projective, Scalar};
    use core::ops::AddAssign;
    use ff::Field;
    use serde::*;

    #[derive(Serialize, Deserialize)]
    pub struct TestEntry {
        #[serde(with = "VecSerializer")]
        elements: Vec<usize, 16>,
    }
    #[test]
    fn test_vecserializer() {
        let mut elements: Vec<usize, 16> = Vec::new();
        for i in 0..16 {
            elements.push(i).unwrap();
        }
        let entry = TestEntry { elements };
        let json = serde_json::to_string(&entry).unwrap();
        assert_eq!(
            "{\"elements\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15]}",
            json
        );

        let entry: serde_json::Result<TestEntry> = serde_json::from_str(json.as_str());
        assert!(entry.is_ok());
        let entry = entry.unwrap();
        assert_eq!(16, entry.elements.len())
    }

    #[test]
    fn test_hash_to_scalar() {
        let h = [0_u8; 32];
        let s = hash_to_scalar(h);
        let json = serde_json::to_string(&s).unwrap();

        assert_eq!("[160,84,69,101,172,192,124,136,195,15,143,221,52,248,200,68,20,107,31,204,72,82,210,99,179,124,0,98,226,99,1,55]", json.as_str())
    }

    #[test]
    fn test_scalar_to_from_bytes() {
        let b = [0_u8; 32];
        let mut s = scalar_from_bytes(&b).unwrap();
        assert!(s.is_zero());

        s.add_assign(&Scalar::from(0xbadc0de));
        let s = s.pow(&[0xff, 0xff, 0xff, 0xff]);

        let bytes = scalar_to_bytes(s);
        assert_eq!(32, bytes.len());

        let json = serde_json::to_string(&bytes).unwrap();

        assert_eq!("[34,107,81,83,198,125,77,125,20,112,145,59,217,231,104,101,219,242,132,240,51,121,145,216,0,107,61,185,157,33,201,37]", json.as_str())
    }

    #[test]
    fn test_sum_of_products() {
        let point = G1Projective::generator();
        let scalar = Scalar::from(0xc0dec0de);
        let sum = sum_of_products(&[point], &mut [scalar]);
        assert_ne!(point, sum);
        assert_ne!(G1Projective::identity(), sum);
    }
}
