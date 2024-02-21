//! Primitives to encode and decode Binary Application Record Encoding (BARE).
//!
//! These primitives are used to encode and decode most performance-sensitive messages
//! in Ockam without using `std`, and not are not meant to support all possible use cases.

/// Defines a set of functions to encode and decode bare encoding
///
/// This module is not dependent on std or any other crate
use crate::compat::vec::Vec;

/// Read a dynamically sized slize from the given cursor
pub fn read_slice<'de>(slice: &'de [u8], index: &mut usize) -> Option<&'de [u8]> {
    let length: usize = read_variable_length_integer(slice, index)?
        .try_into()
        .ok()?;
    if slice.len() - *index >= length {
        let result = &slice[*index..(*index + length)];
        *index += length;
        Some(result)
    } else {
        None
    }
}

/// Returns the size of the encoded slice in bytes
pub fn size_of_slice(slice: &[u8]) -> usize {
    size_of_variable_length(slice.len() as u64) + slice.len()
}

/// Write a dynamically sized slice to the given buffer
pub fn write_slice(destination: &mut Vec<u8>, buffer: &[u8]) {
    write_variable_length_integer(destination, buffer.len() as u64);
    destination.extend_from_slice(buffer);
}

/// Reads a string from the given cursor
pub fn read_str<'de>(slice: &'de [u8], index: &mut usize) -> Option<&'de str> {
    let buffer = read_slice(slice, index)?;
    core::str::from_utf8(buffer).ok()
}

/// Writes a string to the given buffer
pub fn write_str(destination: &mut Vec<u8>, string: &str) {
    write_slice(destination, string.as_bytes());
}

/// Returns the size in bytes of the given variable length integer
pub fn size_of_variable_length(value: u64) -> usize {
    let mut result = 0;
    let mut value = value;
    loop {
        value >>= 7;
        result += 1;
        if value == 0 {
            break;
        }
    }
    result
}

/// Read a variable length integer from the given cursor (ULEB128)
/// returns None if the buffer is too short
pub fn read_variable_length_integer(slice: &[u8], index: &mut usize) -> Option<u64> {
    let mut result = 0;
    let mut shift = 0;
    loop {
        let byte = slice.get(*index)?;
        *index += 1;

        let current = ((byte & 0b0111_1111) as u64) << shift;
        if shift == 63 && *byte != 0b0000_0001 {
            return None;
        }
        result |= current;
        if byte & 0b1000_0000 == 0 {
            break;
        }
        shift += 7;
    }

    Some(result)
}

/// Write a variable length integer to the given buffer (ULEB128)
pub fn write_variable_length_integer(destination: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 0b0111_1111) as u8;
        value >>= 7;
        if value != 0 {
            destination.push(byte | 0b1000_0000);
        } else {
            destination.push(byte);
            break;
        }
    }
}

#[cfg(test)]
mod test {
    use crate::bare::read_variable_length_integer;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn test_variable_length() {
        let values = vec![
            0x0000_0000_0000_0000,
            0x0000_0000_0000_0001,
            0x0000_0000_0000_FFFF,
            0x0000_0000_0001_0000,
            0x0000_0000_FFFF_FFFF,
            0x0000_0001_0000_0000,
            0x0000_FFFF_FFFF_FFFF,
            0x0001_0000_0000_0000,
            0xFFFF_FFFF_FFFF_FFFF,
            0xABCD_EF12_3456_789A,
        ];

        for value in values {
            let mut destination = Vec::new();
            super::write_variable_length_integer(&mut destination, value);
            let result = read_variable_length_integer(&destination, &mut 0).unwrap();
            assert_eq!(value, result);
        }
    }

    #[test]
    fn test_variable_length_fuzzy() {
        let seed = rand::random::<u64>();
        let mut rand = StdRng::seed_from_u64(seed);

        for _ in 0..100 {
            let value = rand.gen::<u64>();
            let mut destination = Vec::new();
            super::write_variable_length_integer(&mut destination, value);
            let result = read_variable_length_integer(&destination, &mut 0).unwrap();
            assert_eq!(
                value, result,
                "{value:#016x} != {result:#016x}. seed: {seed}"
            );
        }
    }

    #[test]
    fn invalid_variable_length_returns_none() {
        let buffer = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        assert_eq!(read_variable_length_integer(&buffer, &mut 0), None);

        let buffer = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x03];
        assert_eq!(read_variable_length_integer(&buffer, &mut 0), None);

        let buffer = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02];
        assert_eq!(read_variable_length_integer(&buffer, &mut 0), None);

        let buffer = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
        assert_eq!(
            read_variable_length_integer(&buffer, &mut 0),
            Some(u64::MAX)
        );
    }

    #[test]
    fn test_size_of_variable_length() {
        let values = vec![
            0x0000_0000_0000_0000,
            0x0000_0000_0000_0001,
            0x0000_0000_0000_007F,
            0x0000_0000_0000_0080,
            0x0000_0000_0000_3FFF,
            0x0000_0000_0000_4000,
            0x0000_0000_001F_FFFF,
            0x0000_0001_0020_0000,
            0xFFFF_FFFF_FFFF_FFFF,
        ];

        for value in values {
            let mut buffer = Vec::new();
            super::write_variable_length_integer(&mut buffer, value);
            let result = super::size_of_variable_length(value);
            assert_eq!(buffer.len(), result, "{value:#016x}");
        }
    }

    #[test]
    fn test_size_of_variable_length_fuzzy() {
        let seed = rand::random::<u64>();
        let mut rand = StdRng::seed_from_u64(seed);

        for _ in 0..100 {
            let value = rand.gen::<u64>();
            let buffer = {
                let mut buffer = Vec::new();
                super::write_variable_length_integer(&mut buffer, value);
                buffer
            };
            let result = super::size_of_variable_length(value);
            let len = buffer.len();
            assert_eq!(len, result, "{len:#016x} != {value:#016x}. seed: {seed}");
        }
    }

    #[test]
    fn test_buffer() {
        let values = vec![
            "hello".as_bytes().to_vec(),
            {
                let mut vec = Vec::with_capacity(200);
                vec.extend((0..200).map(|_| rand::random::<u8>()));
                vec
            },
            {
                let mut vec = Vec::with_capacity(60 * 1024);
                vec.extend((0..60 * 1024).map(|_| rand::random::<u8>()));
                vec
            },
        ];

        for value in values {
            let mut destination = Vec::new();
            super::write_slice(&mut destination, value.as_slice());
            let result = super::read_slice(&destination, &mut 0).unwrap();
            assert_eq!(value.as_slice(), result);
        }
    }

    #[test]
    fn test_buffer_fuzzy() {
        let seed = rand::random::<u64>();
        let mut rand = StdRng::seed_from_u64(seed);

        for _ in 0..10 {
            let value = {
                let size = rand.gen_range(0..90 * 1024);
                let mut vec = Vec::with_capacity(size);
                vec.resize_with(size, || rand.gen());
                vec
            };
            let mut destination = Vec::new();
            super::write_slice(&mut destination, value.as_slice());
            let result = super::read_slice(&destination, &mut 0).unwrap();
            assert_eq!(value.as_slice(), result, "seed: {seed}");
        }
    }
}
