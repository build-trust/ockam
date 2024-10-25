use clap::Args;
use clap::{ArgAction, ValueEnum};
use colorful::Colorful;
use ockam_api::output::OutputFormat;
use ockam_api::{fmt_info, fmt_warn};
use std::fmt::Display;

/// Those arguments are common to all commands
#[derive(Debug, Clone, Args, Default)]
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

    /// Do not write messages to stderr and disable confirmation prompts.
    /// This is useful for scripting and automation, where you don't want the process to block on stdin.
    #[arg(global = true, long, short, env = "QUIET")]
    pub quiet: bool,

    /// Increase verbosity of trace messages
    #[arg(
    global = true,
    long,
    short,
    long_help("Increase verbosity of trace messages by repeating the flag. Use `-v` to show command's \
    info messages, `-vv` will broaden the scope to all ockam crates. Use `-vvv` and `-vvvv` to \
    increase the verbosity to debug and trace respectively"),
    action = ArgAction::Count
    )]
    pub verbose: u8,

    /// Disable colors in output
    #[arg(global = true, long, env = "NO_COLOR")]
    pub no_color: bool,

    /// Disable tty functionality, like interactive prompts.
    #[arg(global = true, long, env = "NO_INPUT")]
    pub no_input: bool,

    /// Specifies the output format of the command. Defaults to 'plain' if not explicitly set.
    /// The 'plain' format is a piece of plain text, the content of which may change based on whether
    /// the stdout is a tty or not. For instance, if stdout is redirected to a file, the output
    /// is usually an identifier that can be used as input for other commands. If stdout is a tty,
    /// the output will contain human-readable information about the command execution.
    /// The 'json' format can be customized with the `--jq` and `--compact-output` options.
    #[arg(global = true, long = "output", value_enum)]
    pub(crate) output_format: Option<OutputFormatArg>,

    /// jq query to apply to the JSON output of the command
    #[arg(global = true, long = "jq")]
    jq_query: Option<String>,

    /// Compact the JSON output of the command
    #[arg(global = true, long)]
    compact_output: bool,

    /// [DEPRECATED] Use `--compact-output` instead
    #[arg(global = true, long, hide = true)]
    pretty: bool,

    // if test_argument_parser is true, command arguments are checked
    // but the command is not executed.
    #[arg(global = true, long, hide = true)]
    pub test_argument_parser: bool,
}

impl GlobalArgs {
    pub fn set_quiet(&self) -> Self {
        let mut clone = self.clone();
        clone.quiet = true;
        clone
    }

    pub fn output_format(&self) -> OutputFormat {
        if self.pretty {
            eprintln!(
                "{}",
                fmt_warn!("The `--pretty` flag is deprecated and has no effect.")
            );
            eprintln!("{}", fmt_info!("The JSON output is now pretty printed by default. Use `--compact-output` to display the compact format."));
        }

        match &self.output_format {
            // If a json related argument is set, assume the output format is json
            None if self.jq_query.is_some() || self.compact_output => OutputFormat::Json {
                jq_query: self.jq_query.clone(),
                compact: self.compact_output,
            },
            Some(OutputFormatArg::Json) => OutputFormat::Json {
                jq_query: self.jq_query.clone(),
                compact: self.compact_output,
            },
            _ => OutputFormat::Plain,
        }
    }
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum OutputFormatArg {
    Plain,
    Json,
}

impl Display for OutputFormatArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormatArg::Plain => write!(f, "plain"),
            OutputFormatArg::Json => write!(f, "json"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format() {
        // default: output_format is set to plain
        let args = GlobalArgs::default();
        assert_eq!(args.output_format(), OutputFormat::Plain);

        // output_format is set to json explicitly
        let args = GlobalArgs {
            output_format: Some(OutputFormatArg::Json),
            jq_query: None,
            compact_output: false,
            ..Default::default()
        };
        assert_eq!(
            args.output_format(),
            OutputFormat::Json {
                jq_query: None,
                compact: false
            }
        );

        // output_format is set to json implicitly
        let args = GlobalArgs {
            output_format: None,
            jq_query: Some(".foo".to_string()),
            compact_output: false,
            ..Default::default()
        };
        assert_eq!(
            args.output_format(),
            OutputFormat::Json {
                jq_query: Some(".foo".to_string()),
                compact: false
            }
        );

        let args = GlobalArgs {
            output_format: None,
            jq_query: None,
            compact_output: true,
            ..Default::default()
        };
        assert_eq!(
            args.output_format(),
            OutputFormat::Json {
                jq_query: None,
                compact: true
            }
        );

        // output_format is set to plain; ignore json related arguments
        let args = GlobalArgs {
            output_format: Some(OutputFormatArg::Plain),
            jq_query: Some(".foo".to_string()),
            compact_output: true,
            ..Default::default()
        };
        assert_eq!(args.output_format(), OutputFormat::Plain);
    }
}
