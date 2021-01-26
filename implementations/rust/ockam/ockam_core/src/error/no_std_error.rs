#[derive(Debug)]
pub struct Error {
    code: u32,
}

impl Error {
    pub fn new(code: u32) -> Self {
        Self { code }
    }

    pub fn code(&self) -> u32 {
        self.code
    }
}
