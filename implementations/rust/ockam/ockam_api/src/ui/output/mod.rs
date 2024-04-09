mod encode_format;
mod ockam_abac;
mod output_format;
mod utils;

pub use encode_format::EncodeFormat;
pub use output_format::OutputFormat;
pub use utils::*;

use crate::Result;

use ockam_core::api::Reply;
use std::fmt::Write;

/// Trait to control how a given type will be printed in the UI layer.
///
/// The `Output` allows us to reuse the same formatting logic for a given type.
///
/// Note that we can't just implement the `Display` trait because most of the types we want
/// to output are defined in other crates or contain fields we want to hide or modify.
/// We can still reuse the `Display` implementation if it's available and already formats
/// the type as we want. For example:
///
/// ```ignore
/// struct MyType;
///
/// impl std::fmt::Display for MyType {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "MyType")
///     }
/// }
///
/// impl Output for MyType {
///     fn single(&self) -> Result<String> {
///         Ok(self.to_string())
///     }
/// }
/// ```
pub trait Output {
    fn single(&self) -> Result<String>;

    fn list(&self) -> Result<String> {
        self.single()
    }
}

impl Output for String {
    fn single(&self) -> Result<String> {
        Ok(self.clone())
    }
}

impl Output for &str {
    fn single(&self) -> Result<String> {
        Ok(self.to_string())
    }
}

impl Output for Vec<u8> {
    fn single(&self) -> Result<String> {
        Ok(hex::encode(self))
    }
}

impl<T: Output> Output for Reply<T> {
    fn single(&self) -> Result<String> {
        match self {
            Reply::Successful(t) => t.single(),
            Reply::Failed(e, status) => {
                let mut output = String::new();
                if let Some(m) = e.message() {
                    writeln!(output, "Failed request: {m}")?;
                } else {
                    writeln!(output, "Failed request")?;
                };
                if let Some(status) = status {
                    writeln!(output, "status: {status}")?;
                }
                Ok(output)
            }
        }
    }
}
