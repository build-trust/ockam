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
        // prepend the length of the mesage to the vector and try
        // again.  I know it's bad, but as long as we don't have
        // properly specified payload encoding this is what will
        // have to do.
        let mut new_v = vec![vec.len() as u8]; // FIXME: does not handle message sizes over 255
        trace!("New message length: {:?}", new_v);

        new_v.append(&mut vec.to_vec());
        M::decode(&new_v)
    })
}
