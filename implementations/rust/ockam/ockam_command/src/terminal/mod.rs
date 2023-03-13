use std::fmt::{Debug, Display};
use std::io::Write;
use std::time::Duration;

use anyhow::{anyhow, Context as _};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

use mode::*;

use crate::{OutputFormat, Result};

pub mod term;

/// A terminal abstraction to handle commands' output and messages styling.
#[derive(Clone)]
pub struct Terminal<T: TerminalWriter, WriteMode = Logging> {
    stdout: T,
    stderr: T,
    quiet: bool,
    no_input: bool,
    output_format: OutputFormat,
    mode: WriteMode,
}

pub enum TerminalBackground {
    Light,
    Dark,
    Unknown,
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
        match std::env::var("COLORFGBG") {
            Ok(v) => {
                let parts: Vec<&str> = v.split(';').collect();
                match parts.get(1) {
                    Some(p) => match p.to_string().parse::<i32>() {
                        Ok(i) => {
                            if (0..8).contains(&i) {
                                TerminalBackground::Dark
                            } else {
                                TerminalBackground::Light
                            }
                        }
                        Err(_e) => TerminalBackground::Unknown,
                    },
                    None => TerminalBackground::Unknown,
                }
            }
            Err(_e) => TerminalBackground::Unknown,
        }
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
    fn prepare_msg(&self, msg: &str) -> Result<String> {
        let mut buffer = Vec::new();
        write!(buffer, "{}", msg)?;
        if self.no_color {
            buffer = strip_ansi_escapes::strip(&buffer)?;
        }
        Ok(String::from_utf8(buffer).context("Invalid UTF-8")?)
    }
}

/// The possible states of Terminal. Each state defines what
/// methods can be used on a given instance.
pub mod mode {
    use super::Output;

    /// Write mode used when writing to the stderr stream.
    #[derive(Clone)]
    pub struct Logging;

    /// Write mode used when writing to the stdout stream.
    #[derive(Clone)]
    pub struct Finished {
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

    fn write(&mut self, s: &str) -> Result<()>;
    fn rewrite(&mut self, s: &str) -> Result<()>;
    fn write_line(&mut self, s: &str) -> Result<()>;
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
            mode: Logging,
        }
    }

    pub fn can_ask_for_user_input(&self) -> bool {
        !self.no_input && self.stderr.is_tty()
    }

    fn should_disable_color(no_color: bool) -> bool {
        // If global argument `--no-color` is passed or the `NO_COLOR` env var is set, colors
        // will be stripped out from output messages. Otherwise, let the terminal decide.
        no_color || std::env::var("NO_COLOR").is_ok()
    }

    fn should_disable_user_input(no_input: bool) -> bool {
        // If global argument `--no-input` is passed or the `NO_INPUT` env var is set we won't be able
        // to ask the user for input.  Otherwise, let the terminal decide based on the `is_tty` value
        no_input || std::env::var("NO_INPUT").is_ok()
    }
}

// Logging mode
impl<W: TerminalWriter> Terminal<W, Logging> {
    pub fn write(&mut self, msg: &str) -> Result<()> {
        if self.quiet {
            return Ok(());
        }
        self.stderr.write(msg)
    }

    pub fn rewrite(&mut self, msg: &str) -> Result<()> {
        if self.quiet {
            return Ok(());
        }
        self.stderr.rewrite(msg)
    }

    pub fn write_line(&mut self, msg: &str) -> Result<()> {
        if self.quiet {
            return Ok(());
        }
        self.stderr.write_line(msg)
    }

    pub fn stdout(self) -> Terminal<W, Finished> {
        Terminal {
            stdout: self.stdout,
            stderr: self.stderr,
            quiet: self.quiet,
            no_input: self.no_input,
            output_format: self.output_format,
            mode: Finished {
                output: Output::new(),
            },
        }
    }
}

// Finished mode
impl<W: TerminalWriter> Terminal<W, Finished> {
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

    pub fn write_line(mut self) -> Result<()> {
        if self.quiet {
            return Ok(());
        }

        // Check that there is at least one output format defined
        if self.mode.output.plain.is_none()
            && self.mode.output.machine.is_none()
            && self.mode.output.json.is_none()
        {
            return Err(anyhow!("At least one output format must be defined").into());
        }

        let plain = self.mode.output.plain.as_ref();
        let machine = self.mode.output.machine.as_ref();
        let json = self.mode.output.json.as_ref();

        let msg = match self.output_format {
            OutputFormat::Plain => {
                if self.stdout.is_tty() {
                    // If not set, fallback with the following priority: Machine -> JSON
                    plain.unwrap_or(
                        machine.unwrap_or(json.context("JSON output should be defined")?),
                    )
                } else {
                    // If not set, fallback with the following priority: JSON -> Plain
                    machine
                        .unwrap_or(json.unwrap_or(plain.context("Plain output should be defined")?))
                }
            }
            // If not set, no fallback is provided and returns an error
            OutputFormat::Json => json.context("JSON output is not defined for this command")?,
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
        let pb = ProgressBar::new_spinner();
        pb.set_draw_target(ProgressDrawTarget::stderr());
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_style(
            ProgressStyle::with_template("{spinner} {msg}")
                .expect("Failed to set progress bar template")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        Some(pb)
    }
}
