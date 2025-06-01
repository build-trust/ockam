use anyhow::anyhow;
use std::process::Command;
use std::str;
use std::str::FromStr;

use ockam::Result;
use ockam_api::authenticator::one_time_code::OneTimeCode;
use ockam_api::cli_state::ExportedEnrollmentTicket;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

/// Invoke the `ockam` command line in order to create a one-time token for
/// a given attribute name/value (and the default project on this machine)
pub async fn create_token(attribute_name: &str, attribute_value: &str) -> Result<OneTimeCode> {
    let token_output = Command::new("ockam")
        .args(vec![
            "project",
            "ticket",
            "--attribute",
            format!("{attribute_name}={attribute_value}").as_str(),
        ])
        .env_remove("OCKAM_LOGGING") // make sure that OCKAM_LOGGING is not set, otherwise the output will contain more than the token
        .output()
        .map_err(|e| error(format!("could not run the `ockam project ticket` successfully: {e:?}")))?;

    // we unwrap the result of decoding the token as UTF-8 since it should be some valid UTF-8 string
    let token_string = str::from_utf8(token_output.stdout.as_slice()).unwrap().trim();

    let decoded =
        ExportedEnrollmentTicket::from_str(token_string).map_err(|e| error(format!("could not decode token: {e}")))?;
    let ticket = decoded.import().await?;

    Ok(ticket.one_time_code)
}

fn error(message: String) -> Error {
    Error::new(Origin::Application, Kind::Invalid, anyhow!(message))
}
