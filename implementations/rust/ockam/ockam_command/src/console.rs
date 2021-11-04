use crate::AppError;
use log::*;

#[derive(Default)]
pub struct Console;

impl Console {
    pub fn error(&self, error: &AppError) {
        error!("{}", error)
    }
}
