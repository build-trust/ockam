/// Compute Internet checksum for arbitrary binary
pub(crate) struct ChecksumAccumulator {
    current_value: u16,
    odd_byte: Option<u8>,
}

impl ChecksumAccumulator {
    /// Constructor
    pub(crate) fn new() -> Self {
        Self {
            current_value: 0,
            odd_byte: None,
        }
    }

    /// Return the checksum value (native-endian)
    pub(crate) fn finalize(mut self) -> u16 {
        if let Some(odd_byte) = self.odd_byte.take() {
            self.update_current_value(u16::from_be_bytes([odd_byte, 0]) as u64)
        }
        self.current_value
    }

    /// Add more data to the checksum. Data is treated like big-endian
    pub(crate) fn add_data(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        let mut sum: u64;

        let data = match self.odd_byte.take() {
            Some(odd_byte) => {
                sum = u16::from_be_bytes([odd_byte, data[0]]) as u64;
                &data[1..]
            }
            None => {
                sum = 0;
                data
            }
        };

        for chunk in data.chunks(2) {
            match chunk {
                [byte] => {
                    self.odd_byte = Some(*byte);
                }
                [byte1, byte2] => {
                    let next = u16::from_be_bytes([*byte1, *byte2]) as u64;

                    // Input length shouldn't be longer than 2^16, so this can't overflow
                    sum += next;
                }
                _ => unreachable!(),
            }
        }

        self.update_current_value(sum);
    }

    fn update_current_value(&mut self, sum: u64) {
        self.current_value = Self::fold((!self.current_value) as u64 + sum);
    }

    /// Converts a checksum into u16 according to 1's complement addition
    pub(crate) fn fold(mut csum: u64) -> u16 {
        for _i in 0..4 {
            if (csum >> 16) > 0 {
                csum = (csum & 0xffff) + (csum >> 16);
            }
        }
        !(csum as u16)
    }
}
