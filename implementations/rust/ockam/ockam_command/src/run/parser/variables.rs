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
    pub fn resolve(contents: &str) -> Result<String> {
        let self_ = serde_yaml::from_str::<Variables>(contents).into_diagnostic()?;
        self_.load()?;
        shellexpand::env(&contents)
            .map(|c| c.to_string())
            .map_err(|e| {
                miette!(
                    "Failed to resolve variable {}: {}",
                    color_primary(&e.var_name),
                    e.cause
                )
            })
    }

    /// Loads the variables into the environment, giving preference to variables set externally.
    /// That is, if one of the variables already exists, it will use the existing value.
    fn load(&self) -> Result<()> {
        if let Some(vars) = &self.variables {
            for (k, v) in vars {
                if std::env::var(k).is_ok() {
                    warn!("Loading variable '{k}' from environment");
                    eprintln!("{}", fmt_warn!("Loading variable '{k}' from environment"));
                    continue;
                }
                let v = v.to_string();
                std::env::set_var(k, v);
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
        let input = r#"
            variables:
              var_s: $MY_ENV_VAR
              var_b: true
              var_i: 1

            nodes:
              - $MY_ENV_VAR
              - is_${var_b}
              - num_${var_i}
        "#;
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
        let resolved = Variables::resolve(input).unwrap();
        assert_eq!(resolved, expected);
    }

    #[test]
    fn give_preference_to_external_variables() {
        std::env::set_var("my_var", "external");
        let input = r#"
            variables:
              my_var: local

            nodes:
              - ${my_var}
        "#;
        let expected = r#"
            variables:
              my_var: local

            nodes:
              - external
        "#;
        let resolved = Variables::resolve(input).unwrap();
        assert_eq!(resolved, expected);
    }

    #[test]
    fn fail_if_unknown_variable() {
        let input = r#"
            variables:
              my_var: local

            nodes:
              - is_${other_var}
        "#;
        let resolved = Variables::resolve(input);
        assert!(resolved.is_err());
    }
}
