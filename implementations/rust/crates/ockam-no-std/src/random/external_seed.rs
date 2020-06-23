use crate::error::OckamResult;
use crate::random::{Random, Seed, ERROR_INVALID_SIZE, MAX_SIZE};

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

// A basic SeedableRandom.
pub struct ExternalSeededRandom {
    seed: Seed,
}

/// A Random that requires a seed to be provided from an external entropy source.
///
/// There is no internal entropy source for seed data accessible to this trait,
/// which requires an external entropy source, such as a hardware random number generator,
/// to provide a seed.
/// [`new`]: SeedableRandom::new
pub trait SeedableRandom: Random {
    fn new(seed: Seed) -> Self;
}

impl SeedableRandom for ExternalSeededRandom {
    fn new(seed: Seed) -> Self {
        ExternalSeededRandom { seed }
    }
}

impl Random for ExternalSeededRandom {
    // Fills the given byte array with random data.
    //
    // Internally, this method utilizes the standard RNG from
    // the rand crate. rand::rngs::StdRng.
    // [rand]: https://crates.io/crates/rand
    //
    // The given array size must be less than or equal to random::MAX_SIZE
    fn get_bytes(&self, bytes: &mut [u8]) -> OckamResult<()> {
        if bytes.len() > MAX_SIZE {
            return Err(ERROR_INVALID_SIZE);
        }

        let mut rng: StdRng = StdRng::from_seed(self.seed);
        rng.fill_bytes(bytes);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::random::external_seed::*;

    #[test]
    fn can_generate_random_number_success() {
        let seed = [
            0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 5, 0, 0, 0, 6, 0, 0, 0, 7,
            0, 0, 0,
        ];

        let random = ExternalSeededRandom::new(seed);

        let mut bytes = [0; 64];
        match random.get_bytes(&mut bytes) {
            Ok(_) => {
                assert!(bytes.iter().any(|x| *x > 0u8));
            }
            Err(_e) => assert!(false),
        }
    }
}
