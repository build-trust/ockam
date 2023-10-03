use std::fmt::Write as _;
use std::fmt::{Debug, Display};
use std::io::Write;
use std::time::Duration;

use colorful::Colorful;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use tokio::sync::Mutex;
use tokio::time::sleep;

pub use colors::*;
pub use fmt::*;
use mode::*;
use ockam_core::env::{get_env, get_env_with_default, FromString};
use ockam_core::errcode::Kind;

use crate::error::Error;
use crate::{fmt_list, fmt_log, fmt_warn, OutputFormat, Result};

pub mod colors;
pub mod fmt;
pub mod term;

/// A terminal abstraction to handle commands' output and messages styling.
#[derive(Clone)]
pub struct Terminal<T: TerminalWriter, WriteMode = ToStdErr> {
    stdout: T,
    stderr: T,
    quiet: bool,
    no_input: bool,
    output_format: OutputFormat,
    mode: WriteMode,
}

impl<T: TerminalWriter, W> Terminal<T, W> {
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }
}

impl<W: TerminalWriter> Default for Terminal<W> {
    fn default() -> Self {
        Terminal::new(false, false, false, OutputFormat::Plain)
    }
}

pub enum ConfirmResult {
    Yes,
    No,
    NonTTY,
}

impl From<bool> for ConfirmResult {
    fn from(value: bool) -> Self {
        if value {
            ConfirmResult::Yes
        } else {
            ConfirmResult::No
        }
    }
}

pub enum TerminalBackground {
    Light,
    Dark,
    Unknown,
}

struct Color(u8);

impl FromString for Color {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        Ok(Color(s.to_string().parse::<u8>().map_err(|_| {
            ockam_core::Error::new(
                ockam_core::errcode::Origin::Core,
                Kind::Internal,
                "u8 parse error",
            )
        })?))
    }
}

struct TerminalColors {
    #[allow(dead_code)]
    foreground: Color,
    background: Color,
}

impl TerminalColors {
    pub fn terminal_background(&self) -> TerminalBackground {
        if (0..8).contains(&self.background.0) {
            TerminalBackground::Dark
        } else {
            TerminalBackground::Light
        }
    }
}

impl FromString for TerminalColors {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        let parts: Vec<&str> = s.split(';').collect();
        Ok(TerminalColors {
            foreground: Color::from_string(parts[0])?,
            background: Color::from_string(parts[1])?,
        })
    }
}

impl TerminalBackground {
    /// Detect if terminal background is "light", "dark" or "unknown".
    ///
    /// There are lots of complex heuristics to check this but they all seem
    /// to work in some cases and fail in others. We want to degrade gracefully.
    /// So we rely on the simple tool of whether the COLORFGBG variable is set.
    ///
    /// If it is set, it usually takes the form <foreground-color>:<background-color>
    /// and if <background-color> is in {0,1,2,3,4,5,6,8}, then we assume the terminal
    /// has a dark background.
    ///
    /// Reference: https://stackoverflow.com/a/54652367
    pub fn detect_background_color() -> TerminalBackground {
        let terminal_colors = get_env::<TerminalColors>("COLORFGBG");
        if let Ok(Some(terminal_colors)) = terminal_colors {
            return terminal_colors.terminal_background();
        }

        TerminalBackground::Unknown
    }
}

/// A small wrapper around the `Write` trait, enriched with CLI
/// attributes to facilitate output handling.
#[derive(Clone)]
pub struct TerminalStream<T: Write + Debug + Clone> {
    writer: T,
    no_color: bool,
}

impl<T: Write + Debug + Clone> TerminalStream<T> {
    fn prepare_msg(&self, msg: impl AsRef<str>) -> Result<String> {
        let mut buffer = Vec::new();
        write!(buffer, "{}", msg.as_ref())?;
        if self.no_color {
            buffer = strip_ansi_escapes::strip(&buffer);
        }
        Ok(String::from_utf8(buffer)
            .into_diagnostic()
            .context("Invalid UTF-8")?)
    }
}

/// The possible states of Terminal. Each state defines what
/// methods can be used on a given instance.
pub mod mode {
    use super::Output;

    /// Write mode used when writing to the stderr stream.
    #[derive(Clone)]
    pub struct ToStdErr;

    /// Write mode used when writing to the stdout stream.
    #[derive(Clone)]
    pub struct ToStdOut {
        pub output: Output,
    }
}

/// The command's output message to be displayed to the user in various formats
#[derive(Clone)]
pub struct Output {
    plain: Option<String>,
    machine: Option<String>,
    json: Option<String>,
}

impl Output {
    fn new() -> Self {
        Self {
            plain: None,
            machine: None,
            json: None,
        }
    }
}

/// Trait defining the main methods to write messages to a terminal stream.
pub trait TerminalWriter: Clone {
    fn stdout(no_color: bool) -> Self;
    fn stderr(no_color: bool) -> Self;
    fn is_tty(&self) -> bool;

    fn write(&mut self, s: impl AsRef<str>) -> Result<()>;
    fn rewrite(&mut self, s: impl AsRef<str>) -> Result<()>;
    fn write_line(&self, s: impl AsRef<str>) -> Result<()>;
}

// Core functions
impl<W: TerminalWriter> Terminal<W> {
    pub fn new(quiet: bool, no_color: bool, no_input: bool, output_format: OutputFormat) -> Self {
        let no_color = Self::should_disable_color(no_color);
        let no_input = Self::should_disable_user_input(no_input);
        let stdout = W::stdout(no_color);
        let stderr = W::stderr(no_color);
        Self {
            stdout,
            stderr,
            quiet,
            no_input,
            output_format,
            mode: ToStdErr,
        }
    }

    pub fn is_tty(&self) -> bool {
        self.stderr.is_tty()
    }

    pub fn quiet() -> Self {
        Self::new(true, false, false, OutputFormat::Plain)
    }

    /// Prompt the user for a confirmation.
    pub fn confirm(&self, msg: impl AsRef<str>) -> Result<ConfirmResult> {
        if !self.can_ask_for_user_input() {
            return Ok(ConfirmResult::NonTTY);
        }
        Ok(ConfirmResult::from(
            dialoguer::Confirm::new()
                .default(true)
                .show_default(true)
                .with_prompt(fmt_warn!("{}", msg.as_ref()))
                .interact()?,
        ))
    }

    pub fn confirmed_with_flag_or_prompt(
        &self,
        flag: bool,
        prompt_msg: impl AsRef<str>,
    ) -> Result<bool> {
        if flag {
            Ok(true)
        } else {
            // If the confirmation flag is not provided, prompt the user.
            match self.confirm(prompt_msg)? {
                ConfirmResult::Yes => Ok(true),
                ConfirmResult::No => Ok(false),
                ConfirmResult::NonTTY => Err(miette!("Use --yes to confirm").into()),
            }
        }
    }

    fn can_ask_for_user_input(&self) -> bool {
        !self.no_input && self.stderr.is_tty()
    }

    fn should_disable_color(no_color: bool) -> bool {
        // If global argument `--no-color` is passed or the `NO_COLOR` env var is set, colors
        // will be stripped out from output messages. Otherwise, let the terminal decide.
        no_color || get_env_with_default("NO_COLOR", false).unwrap_or(false)
    }

    fn should_disable_user_input(no_input: bool) -> bool {
        // If global argument `--no-input` is passed or the `NO_INPUT` env var is set we won't be able
        // to ask the user for input.  Otherwise, let the terminal decide based on the `is_tty` value
        no_input || get_env_with_default("NO_INPUT", false).unwrap_or(false)
    }

    pub fn set_quiet(&self) -> Self {
        let mut clone = self.clone();
        clone.quiet = true;
        clone
    }
}

// Logging mode
impl<W: TerminalWriter> Terminal<W, ToStdErr> {
    pub fn write(&self, msg: impl AsRef<str>) -> Result<()> {
        if self.quiet {
            return Ok(());
        }
        self.stderr.clone().write(msg)
    }

    pub fn rewrite(&self, msg: impl AsRef<str>) -> Result<()> {
        if self.quiet {
            return Ok(());
        }
        self.stderr.clone().rewrite(msg)
    }

    pub fn write_line(&self, msg: impl AsRef<str>) -> Result<&Self> {
        if self.quiet || !self.stdout.is_tty() || self.output_format != OutputFormat::Plain {
            return Ok(self);
        }

        self.stderr
            .write_line(msg)
            .map_err(|e| Error::new_internal_error("Unable to write to stderr.", &e.to_string()))?;
        Ok(self)
    }

    pub fn build_list(
        &self,
        items: &[impl crate::output::Output],
        header: &str,
        empty_message: &str,
    ) -> Result<String> {
        let mut output = String::new();

        // Display header
        let header_len = header.len();
        let padding = 7;
        writeln!(
            output,
            "{}",
            &fmt_log!("┌{}┐", "─".repeat(header_len + (padding * 2)))
        )?;
        writeln!(
            output,
            "{}",
            &fmt_log!("│{}{header}{}│", " ".repeat(padding), " ".repeat(padding))
        )?;
        writeln!(
            output,
            "{}",
            &fmt_log!("└{}┘\n", "─".repeat(header_len + (padding * 2)))
        )?;

        // Display empty message if items is empty
        if items.is_empty() {
            writeln!(output, "{}", &fmt_warn!("{empty_message}"))?;
            return Ok(output);
        }

        // Display items with alternating colors
        for item in items {
            let item = item.list_output()?;
            item.split('\n').for_each(|line| {
                let _ = writeln!(output, "{}", &fmt_list!("{line}"));
            });
            writeln!(output)?;
        }

        Ok(output)
    }

    pub fn stdout(self) -> Terminal<W, ToStdOut> {
        Terminal {
            stdout: self.stdout,
            stderr: self.stderr,
            quiet: self.quiet,
            no_input: self.no_input,
            output_format: self.output_format,
            mode: ToStdOut {
                output: Output::new(),
            },
        }
    }
}

// Finished mode
impl<W: TerminalWriter> Terminal<W, ToStdOut> {
    pub fn is_tty(&self) -> bool {
        self.stdout.is_tty()
    }

    pub fn plain<T: Display>(mut self, msg: T) -> Self {
        self.mode.output.plain = Some(msg.to_string());
        self
    }

    pub fn machine<T: Display>(mut self, msg: T) -> Self {
        self.mode.output.machine = Some(msg.to_string());
        self
    }

    pub fn json<T: Display>(mut self, msg: T) -> Self {
        self.mode.output.json = Some(msg.to_string());
        self
    }

    pub fn write_line(self) -> Result<()> {
        // Check that there is at least one output format defined
        if self.mode.output.plain.is_none()
            && self.mode.output.machine.is_none()
            && self.mode.output.json.is_none()
        {
            return Err(miette!("At least one output format must be defined").into());
        }

        let plain = self.mode.output.plain.as_ref();
        let machine = self.mode.output.machine.as_ref();
        let json = self.mode.output.json.as_ref();

        let msg = match self.output_format {
            OutputFormat::Plain => {
                if self.stdout.is_tty() {
                    // If not set, fallback with the following priority: Machine -> JSON
                    match (plain, machine, json) {
                        (Some(plain), _, _) => plain,
                        (None, Some(machine), _) => machine,
                        (None, None, Some(json)) => json,
                        _ => unreachable!(),
                    }
                } else {
                    // If not set, fallback with the following priority: JSON -> Plain
                    match (machine, json, plain) {
                        (Some(machine), _, _) => machine,
                        (None, Some(json), _) => json,
                        (None, None, Some(plain)) => plain,
                        _ => unreachable!(),
                    }
                }
            }
            // If not set, no fallback is provided and returns an error
            OutputFormat::Json => {
                json.ok_or(miette!("JSON output is not defined for this command"))?
            }
        };
        self.stdout.write_line(msg)
    }
}

// Extensions
impl<W: TerminalWriter> Terminal<W> {
    pub fn progress_spinner(&self) -> Option<ProgressBar> {
        if self.quiet || !self.stderr.is_tty() {
            return None;
        }
        let ticker = [
            "     ⠋", "     ⠙", "     ⠹", "     ⠸", "     ⠼", "     ⠴", "     ⠦", "     ⠧",
            "     ⠇", "     ⠏",
        ];

        let pb = ProgressBar::new_spinner();
        pb.set_draw_target(ProgressDrawTarget::stderr());
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_style(
            ProgressStyle::with_template("{spinner:.yellow} {msg}")
                .expect("Failed to set progress bar template")
                .tick_strings(&ticker),
        );
        Some(pb)
    }

    pub async fn progress_output(
        &self,
        output_messages: &Vec<String>,
        is_finished: &Mutex<bool>,
    ) -> Result<()> {
        let spinner = self.progress_spinner();

        self.progress_output_with_progress_bar(output_messages, is_finished, spinner.as_ref())
            .await
    }

    pub async fn progress_output_with_progress_bar(
        &self,
        output_messages: &Vec<String>,
        is_finished: &Mutex<bool>,
        progress_bar: Option<&ProgressBar>,
    ) -> Result<()> {
        let mut i = 0;
        let progress_bar = match progress_bar {
            Some(pb) => pb,
            None => return Ok(()),
        };

        loop {
            if *is_finished.lock().await {
                progress_bar.finish_and_clear();
                break;
            }

            progress_bar.set_message(output_messages[i].clone());

            if i >= output_messages.len() - 1 {
                i = 0;
            } else {
                i += 1;
            }

            sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }
}
