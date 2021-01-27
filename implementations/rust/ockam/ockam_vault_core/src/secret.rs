use std::fmt::Debug;
use zeroize::Zeroize;

/// Secret
#[derive(Clone, Debug, Zeroize)]
pub struct Secret {
    index: usize,
}

impl Secret {
    pub fn index(&self) -> usize {
        self.index
    }
}

impl Secret {
    pub fn new(index: usize) -> Self {
        Secret { index }
    }
}
