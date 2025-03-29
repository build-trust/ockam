mod branding;
mod encode_format;
mod ockam_abac;
mod output_format;
mod utils;

pub use branding::OutputBranding;
pub use encode_format::EncodeFormat;
pub use output_format::OutputFormat;
pub use utils::*;

use crate::Result;

use crate::terminal::fmt;
use itertools::Itertools;
use ockam_core::api::Reply;
use std::fmt::{Display, Write};

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
///     fn item(&self) -> Result<String> {
///         Ok(self.to_string())
///     }
/// }
/// ```
pub trait Output {
    /// Format to use when the item is printed as a standalone item
    fn item(&self) -> Result<String>;

    /// Format to use when the item is part of a list.
    /// By default, the list representation is the same as the standalone representation
    /// but removing the padding of each line.
    fn as_list_item(&self) -> Result<String> {
        Ok(self
            .item()?
            .lines()
            .map(|l| l.strip_prefix(fmt::PADDING).unwrap_or(l))
            .join("\n"))
    }

    /// Adds padding to each line of the Display output
    fn padded_display(&self) -> String
    where
        Self: Display,
    {
        self.iter_output().pad().to_string()
    }

    /// Returns an iterator over the lines of the Display output
    fn iter_output(&self) -> OutputIter
    where
        Self: Display,
    {
        OutputIter::new(self.to_string())
    }
}

impl Output for String {
    fn item(&self) -> Result<String> {
        Ok(self.clone())
    }
}

impl Output for &str {
    fn item(&self) -> Result<String> {
        Ok(self.to_string())
    }
}

impl Output for Vec<u8> {
    fn item(&self) -> Result<String> {
        Ok(hex::encode(self))
    }
}

impl<T: Output> Output for Reply<T> {
    fn item(&self) -> Result<String> {
        match self {
            Reply::Successful(t) => t.item(),
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

pub struct OutputIter {
    contents: String,
}

impl Display for OutputIter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.contents)
    }
}

impl OutputIter {
    pub fn new(contents: String) -> Self {
        Self { contents }
    }

    pub fn pad(self) -> Self {
        let contents = self
            .contents
            .lines()
            .map(|s| format!("{}{}", fmt::PADDING, s))
            .collect::<Vec<String>>()
            .join("\n");
        Self { contents }
    }

    pub fn indent(self) -> Self {
        let contents = self
            .contents
            .lines()
            .map(|s| format!("{}{}", fmt::INDENTATION, s))
            .collect::<Vec<String>>()
            .join("\n");
        Self { contents }
    }
}
