#[macro_use]
pub mod fmt;
mod highlighting;
pub mod notification;
pub mod term;

pub use fmt::{get_separator_width, ICON_PADDING, INDENTATION, PADDING};
pub use highlighting::TextHighlighter;

use crate::ui::output::OutputFormat;
use crate::{Result, UiError};
use colorful::Colorful;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use jaq_interpret::{Ctx, FilterT, ParseCtx, RcIter, Val};
use miette::{miette, IntoDiagnostic};
use ockam_core::env::get_env_with_default;
use r3bl_rs_utils_core::{ch, ChUnit};
use r3bl_tuify::{get_size, select_from_list, SelectionMode, StyleSheet};
use serde::Serialize;
use std::fmt::Write as _;
use std::fmt::{Debug, Display};
use std::io::Write;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::warn;

/// A terminal abstraction to handle commands' output and messages styling.
#[derive(Clone, Debug)]
pub struct Terminal<T: TerminalWriter + Debug, WriteMode = ToStdErr> {
    stdout: T,
    stderr: T,
    logging_enabled: bool,
    logging_goes_to_file: bool,
    quiet: bool,
    no_input: bool,
    output_format: OutputFormat,
    mode: WriteMode,
    max_width_col_count: usize,
    max_height_row_count: usize,
}

impl<T: TerminalWriter + Debug, W> Terminal<T, W> {
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    fn log_msg(&self, msg: &str) {
        if !self.logging_enabled {
            return;
        }
        for line in msg.lines() {
            let msg = strip_ansi_escapes::strip_str(line);
            let msg = msg
                .trim()
                .trim_start_matches(['✔', '✗', '>', '!'])
                .trim_end_matches('\n')
                .trim();
            if !msg.is_empty() {
                info!("{msg}");
            }
        }
    }
}

/// A small wrapper around the `Write` trait, enriched with CLI
/// attributes to facilitate output handling.
#[derive(Clone, Debug)]
pub struct TerminalStream<T: Write + Debug + Clone> {
    pub writer: T,
    pub no_color: bool,
    bin_name: String,
    brand_name: String,
}

impl<T: Write + Debug + Clone> TerminalStream<T> {
    pub fn prepare_msg(&self, msg: impl AsRef<str>) -> Result<String> {
        let msg = msg.as_ref().to_string();
        let mut msg = if self.brand_name != "Ockam" {
            msg.replace("Ockam", &self.brand_name)
        } else {
            msg
        };
        msg = if self.bin_name != "ockam" {
            msg.replace("ockam", &self.bin_name)
        } else {
            msg
        };

        if self.no_color {
            Ok(strip_ansi_escapes::strip_str(&msg))
        } else {
            Ok(msg)
        }
    }
}

/// Trait defining the main methods to write messages to a terminal stream.
pub trait TerminalWriter: Clone {
    fn stdout(no_color: bool, bin_name: impl Into<String>, brand_name: impl Into<String>) -> Self;
    fn stderr(no_color: bool, bin_name: impl Into<String>, brand_name: impl Into<String>) -> Self;
    fn is_tty(&self) -> bool;
    fn color(&self) -> bool;

    fn write(&mut self, s: impl AsRef<str>) -> Result<()>;
    fn rewrite(&mut self, s: impl AsRef<str>) -> Result<()>;
    fn write_line(&self, s: impl AsRef<str>) -> Result<()>;
}

// Core functions
impl<W: TerminalWriter + Debug> Terminal<W> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        logging_enabled: bool,
        logging_goes_to_file: bool,
        quiet: bool,
        no_color: bool,
        no_input: bool,
        output_format: OutputFormat,
        bin_name: impl Into<String>,
        brand_name: impl Into<String>,
    ) -> Self {
        let bin_name = bin_name.into();
        let brand_name = brand_name.into();
        let no_color = Self::should_disable_color(no_color);
        let no_input = Self::should_disable_user_input(no_input);
        let stdout = W::stdout(no_color, &bin_name, &brand_name);
        let stderr = W::stderr(no_color, bin_name, brand_name);
        let max_width_col_count = get_size().map(|it| it.col_count).unwrap_or(ch!(80)).into();
        Self {
            stdout,
            stderr,
            logging_enabled,
            logging_goes_to_file,
            quiet,
            no_input,
            output_format,
            mode: ToStdErr,
            max_width_col_count,
            max_height_row_count: 5,
        }
    }

    pub fn quiet(logging_enabled: bool, logging_goes_to_file: bool) -> Self {
        Self::new(
            logging_enabled,
            logging_goes_to_file,
            true,
            false,
            false,
            OutputFormat::Plain,
            "ockam",
            "Ockam",
        )
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
                .interact()
                .map_err(UiError::Dialoguer)?,
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
                ConfirmResult::NonTTY => Err(miette!("Use --yes to confirm"))?,
            }
        }
    }

    pub fn confirm_interactively(&self, header: String) -> bool {
        let user_input = select_from_list(
            header,
            ["YES", "NO"].iter().map(|it| it.to_string()).collect(),
            self.max_height_row_count,
            self.max_width_col_count,
            SelectionMode::Single,
            StyleSheet::default(),
        );

        match &user_input {
            Some(it) => it.contains(&"YES".to_string()),
            None => false,
        }
    }

    /// Returns the selected items by the user, or an empty `Vec` if the user did not select any item
    /// or if the user is not able to select an item (e.g. not a TTY, `--no-input` flag, etc.).
    pub fn select_multiple(&self, header: String, items: Vec<String>) -> Vec<String> {
        if !self.can_ask_for_user_input() {
            return Vec::new();
        }

        let user_selected_list = select_from_list(
            header,
            items,
            self.max_height_row_count,
            self.max_width_col_count,
            SelectionMode::Multiple,
            StyleSheet::default(),
        );

        user_selected_list.unwrap_or_default()
    }

    pub fn can_ask_for_user_input(&self) -> bool {
        !self.no_input && self.stderr.is_tty() && !self.quiet
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
impl<W: TerminalWriter + Debug> Terminal<W, ToStdErr> {
    pub fn is_tty(&self) -> bool {
        self.stderr.is_tty()
    }

    /// Return true if log messages are emitted to the console
    fn logging_to_console_only(&self) -> bool {
        self.logging_enabled && !self.logging_goes_to_file
    }

    /// Return true if we can write to stderr
    /// We can write to stderr unless:
    ///  - all the messages are logged to the console
    ///  - or quiet is true
    fn can_write_to_stderr(&self) -> bool {
        !self.logging_to_console_only() && !self.is_quiet()
    }

    pub fn write(&self, msg: impl AsRef<str>) -> Result<()> {
        self.log_msg(msg.as_ref());
        if self.can_write_to_stderr() {
            self.stderr.clone().write(msg)?;
        }
        Ok(())
    }

    pub fn rewrite(&self, msg: impl AsRef<str>) -> Result<()> {
        self.log_msg(msg.as_ref());
        if self.can_write_to_stderr() {
            self.stderr.clone().rewrite(msg)?;
        }
        Ok(())
    }

    pub fn write_line(&self, msg: impl AsRef<str>) -> Result<&Self> {
        self.log_msg(msg.as_ref());
        if self.can_write_to_stderr() {
            self.stderr.write_line(msg)?;
        }
        Ok(self)
    }

    pub fn build_list(
        &self,
        items: &[impl crate::output::Output],
        empty_message: &str,
    ) -> Result<String> {
        // Early return if there are no items to show.
        if items.is_empty() {
            return Ok(fmt_info!("{empty_message}"));
        }

        let mut output = String::new();

        for (idx, item) in items.iter().enumerate() {
            // Add a newline before each item except the first one
            if idx > 0 {
                writeln!(output)?;
            }

            let item = item.as_list_item()?;
            for line in item.lines() {
                writeln!(output, "{}", &fmt_list!("{line}"))?;
            }
        }

        Ok(output)
    }

    pub fn stdout(self) -> Terminal<W, ToStdOut> {
        Terminal {
            stdout: self.stdout,
            stderr: self.stderr,
            logging_enabled: self.logging_enabled,
            logging_goes_to_file: self.logging_goes_to_file,
            quiet: self.quiet,
            no_input: self.no_input,
            output_format: self.output_format,
            mode: ToStdOut {
                output: Output::new(),
            },
            max_width_col_count: self.max_width_col_count,
            max_height_row_count: self.max_height_row_count,
        }
    }
}

// Finished mode
impl<W: TerminalWriter + Debug> Terminal<W, ToStdOut> {
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

    pub fn json_obj<T: Serialize>(mut self, msg: T) -> Result<Self> {
        self.mode.output.json = Some(serde_json::to_value(msg).into_diagnostic()?);
        Ok(self)
    }

    // This function is deprecated in favor of the `json_obj` function above.
    pub fn json<T: Display>(mut self, msg: T) -> Self {
        self.mode.output.json = Some(serde_json::from_str(&msg.to_string()).unwrap());
        self
    }

    /// Return true if log messages are emitted to the console
    fn logging_to_console_only(&self) -> bool {
        self.logging_enabled && !self.logging_goes_to_file
    }

    /// Return true if we can write to stdout
    /// We can write to stdout unless all the messages are logged to the console
    fn can_write_to_stdout(&self) -> bool {
        !self.logging_to_console_only()
    }

    pub fn write_line(mut self) -> Result<()> {
        let msg = match self.mode.output.get_message(
            &self.output_format,
            self.is_tty(),
            self.stdout.color(),
        )? {
            Some(msg) => msg,
            None => return Ok(()),
        };

        // Log the message in the given format and the plain message if present
        match (&msg, self.mode.output.plain.as_ref()) {
            // msg == plain, so we log it once
            (OutputMessage::Plain(msg), Some(_)) => {
                self.log_msg(msg);
            }
            // in any other case, the formats differ, so we log both
            (_, plain) => {
                if let Some(plain) = plain {
                    self.log_msg(plain);
                }
                self.log_msg(msg.as_str());
            }
        }

        // Remove any trailing newline characters.
        // A newline will be added if stdout is a TTY.
        let msg = msg.as_str().trim_end().trim_end_matches('\n');
        if self.can_write_to_stdout() {
            if self.stdout.is_tty() {
                self.stdout.write_line(msg)?;
            } else {
                self.stdout.write(msg)?;
            }
        }
        Ok(())
    }
}

// Extensions
impl<W: TerminalWriter + Debug> Terminal<W> {
    pub fn can_use_progress_bar(&self) -> bool {
        self.stderr.is_tty() && self.can_write_to_stderr()
    }

    pub fn spinner(&self) -> Option<ProgressBar> {
        if !self.can_use_progress_bar() {
            return None;
        }

        let ticker = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
            .into_iter()
            .map(|t| format!("{}{t}", ICON_PADDING))
            .collect::<Vec<String>>();
        let ticker_ref = &ticker.iter().map(|t| t.as_str()).collect::<Vec<&str>>();

        let pb = ProgressBar::new_spinner();
        pb.set_draw_target(ProgressDrawTarget::stderr());
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_style(
            ProgressStyle::with_template("{spinner:.yellow} {msg}")
                .expect("Failed to set progress bar template")
                .tick_strings(ticker_ref),
        );
        Some(pb)
    }

    pub async fn loop_messages(
        &self,
        output_messages: &[String],
        is_finished: &Mutex<bool>,
    ) -> miette::Result<()> {
        if output_messages.is_empty() {
            return Ok(());
        }
        let pb = match self.spinner() {
            Some(pb) => pb,
            None => return Ok(()),
        };

        loop {
            if *is_finished.lock().await {
                break;
            }

            for message in output_messages {
                pb.set_message(message.clone());
                sleep(Duration::from_millis(500)).await;
                if *is_finished.lock().await {
                    break;
                }
            }
        }
        pb.finish_and_clear();
        Ok(())
    }
}

/// Write mode used when writing to the stderr stream.
#[derive(Clone, Debug)]
pub struct ToStdErr;

/// Write mode used when writing to the stdout stream.
#[derive(Clone, Debug)]
pub struct ToStdOut {
    pub(self) output: Output,
}

/// The definition of a command's output messages in different formats.
#[derive(Clone, Debug)]
struct Output {
    plain: Option<String>,
    machine: Option<String>,
    json: Option<serde_json::Value>,
}

impl Output {
    fn new() -> Self {
        Self {
            plain: None,
            machine: None,
            json: None,
        }
    }

    fn get_message(
        &self,
        format: &OutputFormat,
        is_tty: bool,
        color: bool,
    ) -> Result<Option<OutputMessage>> {
        // Check that there is at least one output format defined
        if self.plain.is_none() && self.machine.is_none() && self.json.is_none() {
            return Err(miette!("At least one output format must be defined"))?;
        }

        let plain = self.plain.as_ref();
        let machine = self.machine.as_ref();
        let json = self.json.as_ref();
        let (jq_query, compact) = match format.clone() {
            OutputFormat::Json { jq_query, compact } => (jq_query, compact),
            _ => (None, false),
        };

        // Get the message to be written to stdout
        let msg =
            match format {
                OutputFormat::Plain => {
                    // If interactive, use the following priority: Plain -> Machine -> JSON
                    if is_tty {
                        match (plain, machine, json) {
                            (Some(plain), _, _) => OutputMessage::Plain(plain.clone()),
                            (None, Some(machine), _) => OutputMessage::Machine(machine.clone()),
                            (None, None, Some(json)) => OutputMessage::Json(
                                self.process_json_output(json, jq_query.as_ref(), compact, color)?,
                            ),
                            _ => unreachable!(),
                        }
                    }
                    // If not interactive, use the following priority: Machine -> JSON -> Plain
                    else {
                        match (machine, json, plain) {
                            (Some(machine), _, _) => OutputMessage::Machine(machine.clone()),
                            (None, Some(json), _) => OutputMessage::Json(
                                self.process_json_output(json, jq_query.as_ref(), compact, color)?,
                            ),
                            (None, None, Some(plain)) => OutputMessage::Plain(plain.clone()),
                            _ => unreachable!(),
                        }
                    }
                }
                OutputFormat::Json { .. } => match json {
                    Some(json) => OutputMessage::Json(self.process_json_output(
                        json,
                        jq_query.as_ref(),
                        compact,
                        color,
                    )?),
                    // If not set, no fallback is provided
                    None => {
                        warn!("JSON output is not defined for this command");
                        return Ok(None);
                    }
                },
            };
        Ok(Some(msg))
    }

    fn process_json_output(
        &self,
        json: &serde_json::Value,
        jq_query: Option<&String>,
        compact: bool,
        color: bool,
    ) -> Result<String> {
        let json_string = match jq_query {
            None => self.json_to_string(json, compact)?,
            Some(jq_query) => {
                let filter = {
                    let mut ctx = ParseCtx::new(Vec::new());
                    ctx.insert_natives(jaq_core::core());
                    ctx.insert_defs(jaq_std::std());
                    let (filter, errs) = jaq_parse::parse(jq_query, jaq_parse::main());
                    if !errs.is_empty() {
                        let error_message = errs
                            .iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Err(miette!(error_message))?;
                    }
                    match filter {
                        Some(filter) => ctx.compile(filter),
                        None => return Err(miette!("Failed to parse jq query"))?,
                    }
                };
                let jq_inputs = RcIter::new(core::iter::empty());
                let mut jq_output = filter.run((Ctx::new([], &jq_inputs), Val::from(json.clone())));
                let mut ret = Vec::<serde_json::Value>::new();
                while let Some(Ok(val)) = jq_output.next() {
                    ret.push(val.into());
                }
                let as_string: Vec<String> = ret
                    .iter()
                    .map(|item| self.json_to_string(item, compact))
                    .collect::<Result<Vec<String>>>()?;
                as_string.join("\n")
            }
        };

        let highlighted_json = if color {
            let highlighter = TextHighlighter::new("json")?;
            highlighter.process(&json_string)?
        } else {
            json_string
        };

        Ok(highlighted_json)
    }

    fn json_to_string<T>(&self, json: &T, compact: bool) -> Result<String>
    where
        T: ?Sized + Serialize,
    {
        Ok(if compact {
            serde_json::to_string(&json).into_diagnostic()?
        } else {
            serde_json::to_string_pretty(&json).into_diagnostic()?
        })
    }
}

/// The displayed message written to stdout
#[derive(Clone, Debug, PartialEq)]
enum OutputMessage {
    Plain(String),
    Machine(String),
    Json(String),
}

impl OutputMessage {
    fn as_str(&self) -> &str {
        match self {
            OutputMessage::Plain(msg) => msg,
            OutputMessage::Machine(msg) => msg,
            OutputMessage::Json(msg) => msg,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::compat::rand::random_string;

    #[test]
    fn output_invalid_cases() {
        // No output defined
        let msg = Output::new().get_message(&OutputFormat::Plain, true, false);
        assert!(msg.is_err());

        // If json is requested but it's not defined, no output will be returned
        let output = Output {
            plain: Some("plain".to_string()),
            machine: None,
            json: None,
        };
        let msg = output
            .get_message(
                &OutputFormat::Json {
                    jq_query: None,
                    compact: false,
                },
                true,
                false,
            )
            .unwrap();
        assert!(msg.is_none());
    }

    #[test]
    fn output_to_output_message() {
        let plain = "plain".to_string();
        let machine = "machine".to_string();
        let json = serde_json::json!({ "key": "value" });

        let output = Output {
            plain: Some(plain.clone()),
            machine: Some(machine.clone()),
            json: Some(json.clone()),
        };

        // plain + tty = plain
        let msg = output
            .get_message(&OutputFormat::Plain, true, false)
            .unwrap()
            .unwrap();
        assert_eq!(msg, OutputMessage::Plain(plain));

        // plain + !tty = machine
        let msg = output
            .get_message(&OutputFormat::Plain, false, false)
            .unwrap()
            .unwrap();
        assert_eq!(msg, OutputMessage::Machine(machine));

        // json + _ = json
        let format = OutputFormat::Json {
            jq_query: None,
            compact: false,
        };
        let msg = output.get_message(&format, true, false).unwrap().unwrap();
        assert_eq!(
            msg,
            OutputMessage::Json(serde_json::to_string_pretty(&json).unwrap())
        );
        let msg = output.get_message(&format, false, false).unwrap().unwrap();
        assert_eq!(
            msg,
            OutputMessage::Json(serde_json::to_string_pretty(&json).unwrap())
        );
    }

    #[test]
    fn output_to_output_message_plain_fallbacks() {
        let msg = random_string();
        let json = serde_json::json!({ "key": "value" });

        // plain + tty; plain not defined -> fallback to machine
        let output = Output {
            plain: None,
            machine: Some(msg.clone()),
            json: Some(json.clone()),
        };
        assert_eq!(
            output
                .get_message(&OutputFormat::Plain, true, false)
                .unwrap()
                .unwrap(),
            OutputMessage::Machine(msg.clone())
        );

        // plain + tty; plain and machine not defined -> fallback to json
        let output = Output {
            plain: None,
            machine: None,
            json: Some(json.clone()),
        };
        assert_eq!(
            output
                .get_message(&OutputFormat::Plain, true, false)
                .unwrap()
                .unwrap(),
            OutputMessage::Json(serde_json::to_string_pretty(&json).unwrap())
        );

        // plain + !tty; machine not defined -> fallback to json
        let output = Output {
            plain: Some(msg.clone()),
            machine: None,
            json: Some(json.clone()),
        };
        assert_eq!(
            output
                .get_message(&OutputFormat::Plain, false, false)
                .unwrap()
                .unwrap(),
            OutputMessage::Json(serde_json::to_string_pretty(&json).unwrap())
        );

        // plain + !tty; machine and json not defined -> fallback to plain
        let output = Output {
            plain: Some(msg.clone()),
            machine: None,
            json: None,
        };
        assert_eq!(
            output
                .get_message(&OutputFormat::Plain, false, false)
                .unwrap()
                .unwrap(),
            OutputMessage::Plain(msg.clone())
        );
    }

    #[test]
    fn output_message_json_formatting() {
        let json = serde_json::json!({ "key": "value" });
        let output = Output {
            plain: None,
            machine: None,
            json: Some(json.clone()),
        };

        // pretty, no-color
        assert_eq!(
            output
                .get_message(
                    &OutputFormat::Json {
                        jq_query: None,
                        compact: false
                    },
                    true,
                    false
                )
                .unwrap()
                .unwrap(),
            OutputMessage::Json(serde_json::to_string_pretty(&json).unwrap())
        );

        // pretty, color
        let output_message = output
            .get_message(
                &OutputFormat::Json {
                    jq_query: None,
                    compact: false,
                },
                true,
                true,
            )
            .unwrap()
            .unwrap();
        assert!(output_message.as_str().contains('\u{1b}'));
        assert!(output_message.as_str().contains('\n'));

        // compact, no-color
        assert_eq!(
            output
                .get_message(
                    &OutputFormat::Json {
                        jq_query: None,
                        compact: true
                    },
                    true,
                    false
                )
                .unwrap()
                .unwrap(),
            OutputMessage::Json(serde_json::to_string(&json).unwrap())
        );

        // compact, color
        let output_message = output
            .get_message(
                &OutputFormat::Json {
                    jq_query: None,
                    compact: true,
                },
                true,
                true,
            )
            .unwrap()
            .unwrap();
        assert!(output_message.as_str().contains('\u{1b}'));
        assert!(!output_message.as_str().contains('\n'));
    }
}
