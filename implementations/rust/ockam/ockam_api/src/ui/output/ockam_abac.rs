use crate::colors::color_primary;
use crate::output::Output;
use ockam_abac::{ResourcePolicy, ResourceTypePolicy};

use std::fmt::Write;

impl Output for ResourceTypePolicy {
    fn item(&self) -> crate::Result<String> {
        let mut output = String::new();
        writeln!(
            output,
            "Resource type: {}",
            color_primary(&self.resource_type)
        )?;
        write!(
            output,
            "Expression: {}",
            color_primary(self.expression.to_string())
        )?;
        Ok(output)
    }
}

impl Output for ResourcePolicy {
    fn item(&self) -> crate::Result<String> {
        let mut output = String::new();
        writeln!(
            output,
            "Resource name: {}",
            color_primary(self.resource_name.to_string())
        )?;
        write!(
            output,
            "Expression: {}",
            color_primary(self.expression.to_string())
        )?;
        Ok(output)
    }
}
