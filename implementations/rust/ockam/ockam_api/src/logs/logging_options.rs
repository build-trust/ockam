use nu_ansi_term::{Color, Style};
use ockam_core::env::FromString;
use ockam_core::errcode::{Kind, Origin};
use std::fmt::{Debug, Display, Formatter};
use tracing_core::{Event, Level, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::registry::LookupSpan;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum LoggingEnabled {
    On,
    Off,
}

impl Display for LoggingEnabled {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggingEnabled::On => f.write_str("on"),
            LoggingEnabled::Off => f.write_str("off"),
        }
    }
}

impl FromString for LoggingEnabled {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        FromString::from_string(s).map(|v| {
            if v {
                LoggingEnabled::On
            } else {
                LoggingEnabled::Off
            }
        })
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum GlobalErrorHandler {
    Off,
    Console,
    LogFile,
}

impl Display for GlobalErrorHandler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobalErrorHandler::Off => f.write_str("off"),
            GlobalErrorHandler::Console => f.write_str("console"),
            GlobalErrorHandler::LogFile => f.write_str("logfile"),
        }
    }
}

impl FromString for GlobalErrorHandler {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        match s {
            "off" => Ok(GlobalErrorHandler::Off),
            "console" => Ok(GlobalErrorHandler::Console),
            "logfile" => Ok(GlobalErrorHandler::LogFile),
            _ => Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                format!("incorrect value for the global error handler {s}"),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Colored {
    On,
    Off,
}

/// Options for selecting the log format used in files or in the console
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogFormat {
    Default,
    Pretty,
    Json,
}

impl FromString for LogFormat {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        match s {
            "pretty" => Ok(LogFormat::Pretty),
            "json" => Ok(LogFormat::Json),
            _ => Ok(LogFormat::Default),
        }
    }
}

impl Display for LogFormat {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            LogFormat::Default => write!(f, "default"),
            LogFormat::Pretty => write!(f, "pretty"),
            LogFormat::Json => write!(f, "json"),
        }
    }
}

#[derive(Default)]
pub struct OckamLogFormat {}

impl OckamLogFormat {
    pub fn new() -> Self {
        Self {}
    }

    fn format_timestamp(&self, writer: &mut Writer<'_>) -> std::fmt::Result {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        if writer.has_ansi_escapes() {
            let style = Style::new().dimmed();
            write!(writer, "{}", style.prefix())?;
            write!(writer, "{}", now)?;
            write!(writer, "{} ", style.suffix())?;
        } else {
            write!(writer, "{}", now)?;
            writer.write_char(' ')?;
        }
        Ok(())
    }
}

impl<S, N> FormatEvent<S, N> for OckamLogFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let meta = event.metadata();
        let dimmed = if writer.has_ansi_escapes() {
            Style::new().dimmed()
        } else {
            Style::new()
        };
        let bold = if writer.has_ansi_escapes() {
            Style::new().bold()
        } else {
            Style::new()
        };

        // Timestamp
        self.format_timestamp(&mut writer)?;

        // Level
        let fmt_level = FmtLevel::new(meta.level(), writer.has_ansi_escapes());
        write!(writer, "{} ", fmt_level)?;

        // Event
        ctx.format_fields(writer.by_ref(), event)?;
        writer.write_char(' ')?;

        // Scope
        if let Some(scope) = ctx.event_scope() {
            let mut seen = false;

            for span in scope.from_root() {
                write!(writer, "{}", bold.paint(span.metadata().name()))?;
                seen = true;

                let ext = span.extensions();
                if let Some(fields) = &ext.get::<FormattedFields<N>>() {
                    if !fields.is_empty() {
                        write!(writer, "{}{}{}", bold.paint("{"), fields, bold.paint("}"))?;
                    }
                }
                write!(writer, "{}", dimmed.paint(":"))?;
            }

            if seen {
                writer.write_char(' ')?;
            }
        };

        // Target
        write!(writer, "{} ", dimmed.paint(meta.target()))?;

        // File and line
        let line_number = meta.line();
        if let Some(filename) = meta.file() {
            write!(
                writer,
                "{}{}{}",
                dimmed.paint(filename),
                dimmed.paint(":"),
                if line_number.is_some() { "" } else { " " }
            )?;
        }
        if let Some(line_number) = line_number {
            write!(
                writer,
                "{}{}{}",
                dimmed.prefix(),
                line_number,
                dimmed.suffix()
            )?;
        }

        writeln!(writer)
    }
}

struct FmtLevel<'a> {
    level: &'a Level,
    ansi: bool,
}

impl<'a> FmtLevel<'a> {
    pub(crate) fn new(level: &'a Level, ansi: bool) -> Self {
        Self { level, ansi }
    }
}

const TRACE_STR: &str = "TRACE";
const DEBUG_STR: &str = "DEBUG";
const INFO_STR: &str = " INFO";
const WARN_STR: &str = " WARN";
const ERROR_STR: &str = "ERROR";

impl Display for FmtLevel<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.ansi {
            match *self.level {
                Level::TRACE => write!(f, "{}", Color::Purple.paint(TRACE_STR)),
                Level::DEBUG => write!(f, "{}", Color::Blue.paint(DEBUG_STR)),
                Level::INFO => write!(f, "{}", Color::Green.paint(INFO_STR)),
                Level::WARN => write!(f, "{}", Color::Yellow.paint(WARN_STR)),
                Level::ERROR => write!(f, "{}", Color::Red.paint(ERROR_STR)),
            }
        } else {
            match *self.level {
                Level::TRACE => f.pad(TRACE_STR),
                Level::DEBUG => f.pad(DEBUG_STR),
                Level::INFO => f.pad(INFO_STR),
                Level::WARN => f.pad(WARN_STR),
                Level::ERROR => f.pad(ERROR_STR),
            }
        }
    }
}
