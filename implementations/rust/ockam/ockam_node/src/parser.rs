use ockam_core::{Message, Result};

// TODO: this function can not mutate the data vector it is given, and
// thus copies its contents when using the fallback parsing strategy.
// Ideally fallback parsing could create a slice between the length
// and a vector without needing to move any data.
pub(crate) fn message<M: Message>(vec: &[u8]) -> Result<M> {
    M::decode(vec).or_else(|_| {
        trace!("Parsing payload without inner length...");

        // This condition means we _may_ be dealing with a payload
        // sent by a non-Rust implementation.  In this case we
        // prepend the length of the message to the vector and try
        // again.  I know it's bad, but as long as we don't have
        // properly specified payload encoding this is what we'll
        // have to do.
        let mut new_v = serde_bare::to_vec(&serde_bare::Uint(vec.len() as u64))?;
        trace!("New message length: {:?}", new_v);

        new_v.append(&mut vec.to_vec());
        M::decode(&new_v)
    })
}

/// Test the, unexpected, case where a payload is received that does not
/// code its length at the start. This _may_ happen when dealing with a
/// payload sent by a non-Rust implementation.
/// See https://github.com/build-trust/ockam/issues/2236.
#[test]
fn parse_payload_without_inner_length() {
    // A well formed String payload of 32 smiley chars.
    let payload: [u8; 130] = [
        128, 1, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128,
        240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159,
        152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128,
        240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159,
        152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128,
        240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159,
        152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128,
        240, 159, 152, 128,
    ];
    let r = message::<String>(&payload).unwrap();
    assert_eq!("😀".repeat(32), r);

    // A String payload of 32 smiley chars that is missing its inner length.
    let payload = "😀".repeat(32);
    let r = message::<String>(payload.as_bytes()).unwrap();
    assert_eq!("😀".repeat(32), r);

    // A 100KiB String payload of smiley chars that is missing its inner length.
    let payload = "😀".repeat(25600);
    let r = message::<String>(payload.as_bytes()).unwrap();
    assert_eq!("😀".repeat(25600), r);
}
