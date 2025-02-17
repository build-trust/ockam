//! Implementation of the `TerminalWriter` using the `Term` crate

use crate::output::OutputBranding;
use crate::terminal::{TerminalStream, TerminalWriter};
use crate::Result;
use dialoguer::console::Term;
use std::io::Write;

impl TerminalWriter for TerminalStream<Term> {
    fn stdout(no_color: bool, branding: OutputBranding) -> Self {
        let writer = Term::stdout();
        let no_color = no_color || !writer.features().colors_supported();
        Self {
            writer,
            no_color,
            branding,
        }
    }

    fn stderr(no_color: bool, branding: OutputBranding) -> Self {
        let writer = Term::stderr();
        let no_color = no_color || !writer.features().colors_supported();
        Self {
            writer,
            no_color,
            branding,
        }
    }

    fn is_tty(&self) -> bool {
        self.writer.is_term()
    }

    fn color(&self) -> bool {
        !self.no_color
    }

    fn write(&mut self, s: impl AsRef<str>) -> Result<()> {
        let s = self.prepare_msg(s)?;
        self.writer.write_all(s.as_bytes())?;
        Ok(())
    }

    fn rewrite(&mut self, s: impl AsRef<str>) -> Result<()> {
        let s = self.prepare_msg(s)?;
        self.writer.clear_line()?;
        self.writer.write_all(s.as_bytes())?;
        Ok(())
    }

    fn write_line(&self, s: impl AsRef<str>) -> Result<()> {
        let s = self.prepare_msg(s)?;
        self.writer.write_line(&s)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use colorful::Colorful;
    use dialoguer::console::Term;

    use crate::output::{OutputBranding, OutputFormat};
    use crate::terminal::{LoggingOptions, Terminal, TerminalStream};

    #[test]
    fn test_write() {
        let sut: Terminal<TerminalStream<Term>> = Terminal::new(
            LoggingOptions {
                enabled: false,
                logging_to_file: false,
                with_user_format: false,
            },
            false,
            false,
            false,
            OutputFormat::Plain,
            OutputBranding::default(),
        );
        sut.write("1").unwrap();
        sut.rewrite("1-r\n").unwrap();
        sut.write_line("2".red().to_string()).unwrap();
        sut.to_stdout()
            .plain("This is a human message")
            .machine("This is a machine message")
            .write_line()
            .unwrap();
    }
}
