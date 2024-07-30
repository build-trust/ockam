use clap::builder::TypedValueParser;
use clap::Arg;
use clap::Command;
use hex;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueParser {
    pub changes: Vec<String>,
}

impl ValueParser {
    pub fn from_hex(hex: &str) -> Result<Self, String> {
        match hex::decode(hex) {
            Ok(bytes) => match serde_json::from_slice::<ValueParser>(&bytes) {
                Ok(change_history) => Ok(change_history),
                Err(e) => Err(format!("Failed to deserialize ChangeHistory: {}", e)),
            },
            Err(e) => Err(format!("Failed to decode hex string: {}", e)),
        }
    }

    pub fn to_log_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "Failed to serialize ValueParser".to_string())
    }
}


#[derive(Clone)]
pub struct ChangeHistoryParser;

impl TypedValueParser for ChangeHistoryParser {
    type Value = ValueParser;

    fn parse_ref(
        &self,
        _cmd: &Command,
        _arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value_str = value
            .to_str()
            .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidValue))?;
        ValueParser::from_hex(value_str)
            .map_err(|e| clap::Error::raw(clap::error::ErrorKind::InvalidValue, e))
    }
}