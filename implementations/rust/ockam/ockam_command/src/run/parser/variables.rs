use std::collections::BTreeMap;

use colorful::Colorful;
use miette::{miette, IntoDiagnostic, Result};
use ockam_api::colors::color_primary;
use ockam_api::fmt_warn;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::run::parser::building_blocks::{ArgKey, ArgValue};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Variables {
    pub variables: Option<BTreeMap<ArgKey, ArgValue>>,
}

impl Variables {
    pub fn resolve(contents: &mut String) -> Result<()> {
        let self_ = serde_yaml::from_str::<Variables>(contents).into_diagnostic()?;
        self_.load()?;
        *contents = shellexpand::env(&contents)
            .map(|c| c.to_string())
            .map_err(|e| {
                miette!(
                    "Failed to resolve variable '{}': {}",
                    color_primary(&e.var_name),
                    e.cause
                )
            })?;
        Ok(())
    }

    /// Loads the variables into the environment, giving preference to variables set externally.
    /// That is, if one of the variables already exists, it will use the existing value.
    fn load(&self) -> Result<()> {
        if let Some(vars) = &self.variables {
            for (key, value) in vars {
                if std::env::var(key.as_str()).is_ok() {
                    warn!("Loading variable '{key}' from environment");
                    eprintln!("{}", fmt_warn!("Loading variable '{key}' from environment"));
                    continue;
                }

                let string_value = match value {
                    ArgValue::List(_) => {
                        return Err(miette!(
                            "List values are not supported for variable '{key}'"
                        ));
                    }
                    ArgValue::String(value) => value.to_string(),
                    ArgValue::Bool(value) => value.to_string(),
                    ArgValue::Int(value) => value.to_string(),
                };

                if string_value.is_empty() {
                    return Err(miette!("Empty value for variable '{key}'"));
                }
                std::env::set_var(key.as_str(), string_value);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_variables() {
        std::env::set_var("MY_ENV_VAR", "my_env_value");
        let mut input = r#"
            variables:
              var_s: $MY_ENV_VAR
              var_b: true
              var_i: 1

            nodes:
              - $MY_ENV_VAR
              - is_${var_b}
              - num_${var_i}
        "#
        .to_string();
        let expected = r#"
            variables:
              var_s: my_env_value
              var_b: true
              var_i: 1

            nodes:
              - my_env_value
              - is_true
              - num_1
        "#;
        Variables::resolve(&mut input).unwrap();
        assert_eq!(&input, expected);
        assert_eq!(std::env::var("var_s").unwrap(), "$MY_ENV_VAR");
        assert_eq!(std::env::var("var_b").unwrap(), "true");
        assert_eq!(std::env::var("var_i").unwrap(), "1");
    }

    #[test]
    fn give_preference_to_external_variables() {
        std::env::set_var("my_var", "external");
        let mut input = r#"
            variables:
              my_var: local

            nodes:
              - ${my_var}
        "#
        .to_string();
        let expected = r#"
            variables:
              my_var: local

            nodes:
              - external
        "#;
        Variables::resolve(&mut input).unwrap();
        assert_eq!(&input, expected);
        assert_eq!(std::env::var("my_var").unwrap(), "external");
    }

    #[test]
    fn fail_if_unknown_variable() {
        let mut input = r#"
            variables:
              my_var: local

            nodes:
              - is_${other_var}
        "#
        .to_string();
        let res = Variables::resolve(&mut input);
        assert!(res.is_err());
    }
}
