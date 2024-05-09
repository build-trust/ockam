use clap::ArgAction;
use clap::Args;
use ockam_api::output::OutputFormat;

use ockam_core::env::get_env_with_default;

/// Those arguments are common to all commands
#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
    #[arg(
    global = true,
    long,
    short,
    help("Print help information (-h compact, --help extensive)"),
    long_help("Print help information (-h displays compact help summary, --help displays extensive help summary)"),
    help_heading("Global Options"),
    action = ArgAction::Help
    )]
    help: Option<bool>,

    /// Do not print any log messages to stderr and disable confirmation prompts.
    /// This is useful for scripting and automation, where you don't want the process to block on stdin.
    #[arg(global = true, long, short, default_value_t = quiet_default_value())]
    pub quiet: bool,

    /// Increase verbosity of trace messages
    #[arg(
    global = true,
    long,
    short,
    long_help("Increase verbosity of trace messages by repeating the flag. Use `-v` to show \
    info messages, `-vv` to show debug messages, and `-vvv` to show trace messages"),
    action = ArgAction::Count
    )]
    pub verbose: u8,

    /// Disable colors in output
    #[arg(global = true, long, default_value_t = no_color_default_value())]
    pub no_color: bool,

    /// Disable tty functionality, like interactive prompts.
    #[arg(global = true, long, default_value_t = no_input_default_value())]
    pub no_input: bool,

    /// Specifies the output format of the command. Defaults to 'plain' if not explicitly set.
    /// The 'plain' format is a piece of plain text, the content of which may change based on whether
    /// the stdout is a tty or not. For instance, if stdout is redirected to a file, the output
    /// is usually an identifier that can be used as input for other commands. If stdout is a tty,
    /// the output will contain human-readable information about the command execution.
    /// The 'json' format is a one-line JSON object, which can be made more readable using a tool like jq.
    #[arg(global = true, long = "output", value_enum, default_value = "plain")]
    pub output_format: OutputFormat,

    // if test_argument_parser is true, command arguments are checked
    // but the command is not executed.
    #[arg(global = true, long, hide = true)]
    pub test_argument_parser: bool,
}

fn quiet_default_value() -> bool {
    get_env_with_default("QUIET", false).unwrap_or(false)
}

fn no_color_default_value() -> bool {
    get_env_with_default("NO_COLOR", false).unwrap_or(false)
}

fn no_input_default_value() -> bool {
    get_env_with_default("NO_INPUT", false).unwrap_or(false)
}

impl Default for GlobalArgs {
    fn default() -> Self {
        Self {
            help: None,
            quiet: quiet_default_value(),
            verbose: 0,
            no_color: no_color_default_value(),
            no_input: no_input_default_value(),
            output_format: OutputFormat::Plain,
            test_argument_parser: false,
        }
    }
}

impl GlobalArgs {
    pub fn set_quiet(&self) -> Self {
        let mut clone = self.clone();
        clone.quiet = true;
        clone
    }
}
