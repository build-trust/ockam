use clap::error::{Error, ErrorKind};
use ockam_core::env::parse_duration;
use std::time::Duration;

pub(crate) fn duration_parser(arg: &str) -> Result<Duration, clap::Error> {
    parse_duration(arg).map_err(|_| Error::raw(ErrorKind::InvalidValue, "Invalid duration."))
}
