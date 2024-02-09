use crate::run::parser::resources::{ArgKey, ArgValue};
use miette::{miette, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Variables {
    pub variables: Option<BTreeMap<ArgKey, ArgValue>>,
}

impl Variables {
    const VAR_SIGN: char = '#';
    const VAR_START: char = '{';
    const VAR_END: char = '}';

    pub fn resolve(&self, contents: &str) -> Result<String> {
        let env = self.load()?;
        let mut resolved = contents.to_string();
        for (k, v) in env.iter() {
            let from = format!(
                "{}{}{}{}",
                Variables::VAR_SIGN,
                Variables::VAR_START,
                k,
                Variables::VAR_END
            );
            resolved = resolved.replace(&from, v);
        }
        Ok(resolved)
    }

    fn load(&self) -> Result<BTreeMap<String, String>> {
        let mut env = BTreeMap::new();
        if let Some(vars) = &self.variables {
            for (k, v) in vars {
                let v = v.to_string();
                self.validate(k, &v)?;
                env.insert(k.clone(), v);
            }
        }
        Ok(env)
    }

    fn validate(&self, k: &ArgKey, v: &str) -> Result<()> {
        // Check variable name
        if k.contains([
            Variables::VAR_SIGN,
            Variables::VAR_START,
            Variables::VAR_END,
        ]) {
            return Err(miette!("The variable name '{k}' is not valid"));
        }
        // Check variable value
        if let Some(sign_pos) = v.find(Variables::VAR_SIGN) {
            // if the next character is Variables::VAR_START and
            // contains a Variables::VAR_END after that, then it's a variable
            if v.chars().nth(sign_pos + 1) == Some(Variables::VAR_START) {
                if let Some(end_pos) = v.find(Variables::VAR_END) {
                    if end_pos > sign_pos {
                        return Err(miette!(
                            "The value '{v}' of the variable '{k}' can't contain another variable"
                        ));
                    }
                }
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
        std::env::set_var("VALUE", "my_env_value");
        let yaml = r#"
            variables:
              var_s: $VALUE
              var_b: true
              var_i: 1
        "#;
        let vars = serde_yaml::from_str::<Variables>(yaml).unwrap();
        let template = "#{var_s} is_#{var_b} num_#{var_i}";
        let resolved = vars.resolve(template).unwrap();
        assert_eq!(resolved, "$VALUE is_true num_1");
    }

    #[test]
    fn leave_unknown_variables_unresolved() {
        let yaml = r#"
            variables:
              var_s: value
        "#;
        let vars = serde_yaml::from_str::<Variables>(yaml).unwrap();
        let template = "my_#{var_s} is_#{var_b} num_#{var_i}";
        let resolved = vars.resolve(template).unwrap();
        assert_eq!(resolved, "my_value is_#{var_b} num_#{var_i}");
    }

    #[test]
    fn fail_if_invalid_variable_name() {
        let yaml = r#"
            variables:
              var{}_s: value
        "#;
        let vars = serde_yaml::from_str::<Variables>(yaml).unwrap();
        let template = "";
        let result = vars.resolve(template);
        assert!(result.is_err());
    }

    #[test]
    fn fail_if_invalid_variable_value() {
        let yaml = r#"
            variables:
              var_s: '#{var_s}'
        "#;
        let vars = serde_yaml::from_str::<Variables>(yaml).unwrap();
        let template = "";
        let result = vars.resolve(template);
        assert!(result.is_err());
    }
}
