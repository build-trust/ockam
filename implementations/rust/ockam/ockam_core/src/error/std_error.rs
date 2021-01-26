use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub struct Error {
    code: u32,
    domain: &'static str,
}

impl Error {
    pub fn new(code: u32, domain: &'static str) -> Self {
        Self { code, domain }
    }

    pub fn code(&self) -> u32 {
        self.code
    }

    pub fn domain(&self) -> &'static str {
        &self.domain
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "Error code: {}, domain: {}", self.code, self.domain)
    }
}

impl std::error::Error for Error {}
