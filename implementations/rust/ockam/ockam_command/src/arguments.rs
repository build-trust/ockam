/// Return true if the list of arguments contains a help flag
pub fn has_help_flag(input: &[String]) -> bool {
    input.contains(&"-h".to_string()) || input.contains(&"--help".to_string())
}

/// Return true if the list of arguments contains a version flag
pub fn has_version_flag(input: &[String]) -> bool {
    input.contains(&"-V".to_string()) || input.contains(&"--version".to_string())
}

/// Replaces the '-' placeholder character with a string value coming from stdin
/// This is useful to be able to pipe the output of a command to another command.
///
/// For example:
///
/// ockam secure-channel create --from me --to /node/node-1/service/api |
//     ockam message send hello --from me --to -/service/uppercase
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
