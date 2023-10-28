//! Implementation of the `TerminalWriter` using the `Term` crate

use std::io::Write;

use console::Term;

use crate::terminal::{TerminalStream, TerminalWriter};
use crate::Result;

impl TerminalWriter for TerminalStream<Term> {
    fn stdout(no_color: bool) -> Self {
        let writer = Term::stdout();
        let no_color = no_color || !writer.features().colors_supported();
        Self { writer, no_color }
    }

    fn stderr(no_color: bool) -> Self {
        let writer = Term::stderr();
        let no_color = no_color || !writer.features().colors_supported();
        Self { writer, no_color }
    }

    fn is_tty(&self) -> bool {
        self.writer.is_term()
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

    use crate::terminal::Terminal;
    use crate::OutputFormat;

    use super::*;

    #[test]
    fn test_write() {
        let sut: Terminal<TerminalStream<Term>> =
            Terminal::new(false, false, false, OutputFormat::Plain);
        sut.write("1").unwrap();
        sut.rewrite("1-r\n").unwrap();
        sut.write_line(&"2".red().to_string()).unwrap();
        sut.stdout()
            .plain("This is a human message")
            .machine("This is a machine message")
            .write_line()
            .unwrap();
    }
}
