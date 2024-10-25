/// Compute Internet checksum according to RFC 1071.
pub fn checksum(packet: usize, size: usize, end: usize) -> u16 {
    fold(sum(packet, size, end))
}

/// Checksum update according to RFC 1624.
pub fn checksum_update_word(original_check: u16, old_word: u16, new_word: u16) -> u16 {
    let mut csum = (!original_check) as u64;
    csum += (!old_word) as u64;
    csum += new_word as u64;

    fold(csum)
}

/// Converts a checksum into u16 according to 1's complement addition
fn fold(mut csum: u64) -> u16 {
    for _i in 0..4 {
        if (csum >> 16) > 0 {
            csum = (csum & 0xffff) + (csum >> 16);
        }
    }
    !(csum as u16)
}

/// Simple u16 sum for arbitrary data.
///  WARNING: The data length should a multiple of 2.
fn sum(ptr: usize, size: usize, end: usize) -> u64 {
    let mut res = 0u64;

    let mut p = ptr;

    for _ in 0..size / 2 {
        // we could check the sizing once even before calling this function and omit this check,
        // but it seems like verifier is not clever enough to deduct that it's valid
        // TODO: Check if #[repr(packed)] would help
        if p + 2 > end {
            break;
        }

        res += unsafe { *(p as *const u16) } as u64;
        p += 2;
    }

    res
}
