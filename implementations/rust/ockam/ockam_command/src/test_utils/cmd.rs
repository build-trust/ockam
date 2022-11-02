use crate::test_utils::ockam_bin;
use anyhow::{anyhow, Result};
use duct::unix::HandleExt;
use duct::{cmd, Expression, Handle};
use nix::libc;
use regex::Regex;
use std::process::{ExitStatus, Output};
use std::time::Instant;
use std::{fs, io};
use std::{fs::File, io::Read, thread::sleep, time::Duration};
use tempfile::NamedTempFile;
use thiserror::Error;
use tracing::info;
use tracing_subscriber::fmt::TestWriter;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

const POLL_DURATION: Duration = Duration::from_millis(250);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct CmdBuilder {
    cmd_line: String,
    expr: Expression,
    timeout: Duration,
    stdin: Option<Vec<u8>>,
}

impl std::fmt::Debug for CmdBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CmdBuilder")
            .field("cmd_line", &self.cmd_line)
            .field("timeout", &self.timeout)
            .field("stdin", &self.stdin)
            .finish()
    }
}

impl CmdBuilder {
    pub fn ockam(cmd_line: &str) -> Result<Self> {
        Ok(CmdBuilder {
            cmd_line: String::from(cmd_line),
            expr: cmd(ockam_bin(), shellwords::split(cmd_line)?),
            timeout: DEFAULT_TIMEOUT,
            stdin: None,
        })
    }

    pub fn new(cmd_line: &str) -> Result<Self> {
        let args = shellwords::split(cmd_line)?;
        Ok(CmdBuilder {
            cmd_line: String::from(cmd_line),
            expr: cmd(&args[0], &args[1..]),
            timeout: DEFAULT_TIMEOUT,
            stdin: None,
        })
    }

    /// Set timeout value.
    /// By default, timeout is [`DEFAULT_TIMEOUT`].
    pub fn set_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set stdin for command.
    pub fn set_stdin<T: Into<Vec<u8>>>(mut self, bytes: T) -> Self {
        self.stdin = Some(bytes.into());
        self
    }

    pub fn pipe(mut self, cmd_line: &str) -> Result<Self> {
        let expr = cmd(ockam_bin(), shellwords::split(cmd_line)?);
        self.expr = self.expr.pipe(expr);
        Ok(self)
    }

    fn build(mut self) -> Result<Cmd> {
        self.expr = self.expr.unchecked().stdout_capture().stderr_capture();
        if let Some(bytes) = self.stdin {
            self.expr = self.expr.stdin_bytes(bytes);
        }
        Ok(Cmd::new(self.cmd_line, self.timeout, self.expr))
    }

    /// Run command to completion.
    ///
    /// May timeout.
    ///
    /// On success, returns command's output.
    ///
    /// Consumes the [`CmdBuilder`].
    pub fn run(self) -> Result<Output> {
        info!(cmd = %self.cmd_line, "Running command");
        let mut cmd = self.build()?;
        cmd.start()?;
        info!(cmd = %cmd.cmd_line, "Process spawned");
        let res = cmd.wait()?;
        info!(cmd = %cmd.cmd_line, "Command run successfully");
        Ok(res)
    }
}

struct Cmd {
    cmd_line: String,
    timeout: Duration,
    expr: Expression,
    handle: Option<Handle>,
}

impl Cmd {
    fn new(cmd_line: String, timeout: Duration, expr: Expression) -> Self {
        Cmd {
            cmd_line,
            timeout,
            expr,
            handle: None,
        }
    }

    fn start(&mut self) -> Result<()> {
        self.handle = Some(self.expr.start()?);
        Ok(())
    }

    /// Wait for command to complete.
    /// May timeout.
    /// On success, returns command's output.
    fn wait(&mut self) -> Result<Output> {
        info!(cmd = %self.cmd_line, "Waiting for command to finish");
        let cycles = self.timeout.as_millis() / POLL_DURATION.as_millis();
        for _ in 0..cycles {
            let handle = self.handle.take().unwrap();
            if let Some(result) = handle.try_wait()? {
                info!(cmd = %self.cmd_line, exitcode = ?result.status.code(), "Command finished");
                return Ok(handle.into_output().unwrap());
            } else {
                self.handle = Some(handle);
                sleep(POLL_DURATION);
            }
        }
        Err(anyhow!("Command timed out, cmd=`{}`", self.cmd_line))
    }
}

impl Drop for Cmd {
    /// Cleanup spawned process.
    ///
    /// On unix, send SIGINT (Ctrl-C) to process to allow it the possibility
    /// of shutting itself down and saving its llvm coverage data.
    ///
    /// On non-unix, kill process.
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            if cfg!(unix) {
                let _ = handle.send_signal(libc::SIGINT);
            } else {
                let _ = handle.kill();
            }
        }
    }
}

pub fn read_to_str(c: &[u8]) -> &str {
    std::str::from_utf8(c).unwrap().trim()
}
