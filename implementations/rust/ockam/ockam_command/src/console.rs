use crate::AppError;
use log::*;

pub struct Console;

impl Default for Console {
    fn default() -> Self {
        Self {}
    }
}

impl Console {
    pub fn error(&self, error: &AppError) {
        error!("{}", error)
    }
}
