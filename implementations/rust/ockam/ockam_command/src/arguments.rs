use miette::miette;

/// Return true if the list of arguments contains a help flag
pub fn has_help_flag(input: &[String]) -> bool {
    input.contains(&"-h".to_string()) || input.contains(&"--help".to_string())
}

/// Return true if the list of arguments contains a version flag
pub fn has_version_flag(input: &[String]) -> bool {
    input.contains(&"-V".to_string()) || input.contains(&"--version".to_string())
}

/// Return a list of every `--env KEY=VALUE` attribute parameter in the list of arguments
/// and a list of the remaining arguments
#[allow(clippy::type_complexity)]
pub fn get_env_attributes(
    input: &[String],
) -> miette::Result<Option<(Vec<(String, String)>, Vec<String>)>> {
    let mut attributes = Vec::new();
    let mut remaining = Vec::new();
    let mut iter = input.iter();

    while let Some(arg) = iter.next() {
        if arg == "--env" {
            let key_value = iter
                .next()
                .ok_or_else(|| miette!("Missing value for --env, expected KEY=VALUE"))?;
            let mut key_value = key_value.split('=');
            let key = key_value
                .next()
                .ok_or_else(|| miette!("Missing key for --env, expected KEY=VALUE"))?;
            let value = key_value
                .next()
                .ok_or_else(|| miette!("Missing value for --env, expected KEY=VALUE"))?;
            attributes.push((key.to_string(), value.to_string()));
        } else {
            remaining.push(arg.to_string());
        }
    }

    if attributes.is_empty() {
        Ok(None)
    } else {
        Ok(Some((attributes, remaining)))
    }
}

/// Replaces the '-' placeholder character with a string value coming from stdin
/// This is useful to be able to pipe the output of a command to another command.
///
/// For example:
///
/// ockam secure-channel create --from me --to /node/node-1/service/api |
///     ockam message send hello --from me --to -/service/uppercase
///
pub fn replace_hyphen_with_stdin(s: String) -> String {
    let input_stream = std::io::stdin();
    if s.contains("/-") {
        let mut buffer = String::new();
        input_stream
            .read_line(&mut buffer)
            .expect("could not read from standard input");
        let args_from_stdin = buffer
            .trim()
            .split('/')
            .filter(|&s| !s.is_empty())
            .fold("".to_owned(), |acc, s| format!("{acc}/{s}"));

        s.replace("/-", &args_from_stdin)
    } else if s.contains("-/") {
        let mut buffer = String::new();
        input_stream
            .read_line(&mut buffer)
            .expect("could not read from standard input");

        let args_from_stdin = buffer
            .trim()
            .split('/')
            .filter(|&s| !s.is_empty())
            .fold("/".to_owned(), |acc, s| format!("{acc}{s}/"));

        s.replace("-/", &args_from_stdin)
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn environment_attributes_replaced() {
        let input = vec![
            "ockam".to_string(),
            "--env".to_string(),
            "KEY1=VALUE1".to_string(),
            "command".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
            "--env".to_string(),
            "KEY2=VALUE2".to_string(),
        ];

        let (attributes, remaining) = super::get_env_attributes(&input).unwrap().unwrap();

        assert_eq!(attributes.len(), 2);
        assert_eq!(attributes[0], ("KEY1".to_string(), "VALUE1".to_string()));
        assert_eq!(attributes[1], ("KEY2".to_string(), "VALUE2".to_string()));

        assert_eq!(remaining.len(), 4);
        assert_eq!(remaining[0], "ockam".to_string());
        assert_eq!(remaining[1], "command".to_string());
        assert_eq!(remaining[2], "arg1".to_string());
        assert_eq!(remaining[3], "arg2".to_string());
    }

    #[test]
    pub fn environment_attributes_not_replaced() {
        let input = vec![
            "ockam".to_string(),
            "command".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
        ];

        let result = super::get_env_attributes(&input).unwrap();
        assert!(result.is_none());
    }
}
