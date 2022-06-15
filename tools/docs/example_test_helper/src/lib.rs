//! Helper for the test code of Ockam Examples.
//!
//! Allows test code to spawn or run-to-completeion child processes
//! described by command lines.
//!
//! Processes have these environment variables set...
//! - `OCKAM_LOG=trace`
//!
//! Some functions will timeout if [`DEFAULT_TIMEOUT_MS`] milliseconds
//! pass. In parituclar [`CmdBuilder::run()`], but not [`CmdBuilder::spawn()`].
//!
//! Processes which have not completed are killed (or signalled) when their
//! [`CmdRunner`] is dropped.
//!
//! The stdout of processes are written to temporary files. Those temporary files
//! are left for the operating system to clear up.
//!
//! # Examples
//!
//! Run a command to completion and assert a successful run.
//! ```
//! use example_test_helper::{CmdBuilder};
//!
//! let (exitcode, stdout) = CmdBuilder::new("rustup --version").run().unwrap();
//!
//! assert_eq!(Some(0), exitcode);
//! assert!(stdout.contains("rustup"));
//! ```
//!
//! Spawn a command to run in the background and wait to match a regex on its stdout.
//! If the process is still running when the CmdRunner is dropped it will be killed.
//! Can use [`CmdRunner::wait()`] to wait for a process to complete.
//! ```
//! use example_test_helper::{CmdBuilder};
//!
//! let cmd = CmdBuilder::new("rustup --version").spawn().unwrap();
//!
//! let mut captures = cmd.match_stdout(r"(?im)^rustup (\d+\.\d+.\d+)").unwrap();
//! let version = captures.swap_remove(1).unwrap();
//!
//! assert_eq!(version.matches(".").count(), 2);
//! ```
//!
use duct::unix::HandleExt;
use duct::{cmd, Handle};
use regex::Regex;
use std::time::SystemTime;
use std::{fs, io};
use std::{fs::File, io::Read, thread::sleep, time::Duration};
use tempfile::NamedTempFile;
use thiserror::Error;
use tracing::info;
use tracing_subscriber::fmt::TestWriter;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

const POLL_MS: u32 = 250;
const ENVIRONMENT_VARIABLES: &[(&str, &str)] = &[("OCKAM_LOG", "trace")];

/// Default timeout value in milliseconds.
pub const DEFAULT_TIMEOUT_MS: u32 = 180_000;

type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("timed out")]
    Timeout,
    #[error("io error")]
    Io(#[from] io::Error),
    #[error("regex error")]
    Regex(#[from] regex::Error),
    #[error("shellwords error")]
    Shellwords(#[from] shellwords::MismatchedQuotes),
    #[error("exitcode was non-zero")]
    NonzeroExitcode,
    #[error("command has already exited")]
    ExitedCmd,
    #[error("failed to create temporary file for stdout")]
    StdoutTempfile,
}

pub struct CmdBuilder {
    cmd_line: String,
    timeout_ms: u32,
    stdin: Option<Vec<u8>>,
}

impl std::fmt::Debug for CmdBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CmdBuilder")
            .field("cmd_line", &self.cmd_line)
            .field("timeout_ms", &self.timeout_ms)
            .field("stdin", &self.stdin)
            .finish()
    }
}

impl CmdBuilder {
    /// Create a new instance.
    pub fn new(cmd_line: &str) -> Self {
        // Configure our tracing
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let filter = EnvFilter::default().add_directive(LevelFilter::INFO.into());

            // Ignore errors. Logging is not crucial here.
            // Use TestWriter so 'cargo test' can choose to capture logging as it wants.
            let _ = fmt()
                .with_env_filter(filter)
                .with_writer(TestWriter::new())
                .try_init();
        });

        CmdBuilder {
            cmd_line: String::from(cmd_line),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            stdin: None,
        }
    }

    /// Set timeout value in milliseconds.
    /// By default, timeout is [`DEFAULT_TIMEOUT_MS`].
    pub fn set_timeout(mut self, value_ms: u32) -> Self {
        self.timeout_ms = value_ms;
        self
    }

    /// Set stdin for command.
    pub fn set_stdin<T: Into<Vec<u8>>>(mut self, bytes: T) -> Self {
        self.stdin = Some(bytes.into());
        self
    }

    /// Run command to completion.
    ///
    /// May timeout.
    ///
    /// On success, returns command's exit code and stdout.
    ///
    /// Consumes the [`CmdBuilder`].
    pub fn run(self) -> Result<(Option<i32>, String)> {
        info!("Running '{}'", self.cmd_line);

        // Spawn command
        let runner = self.spawn()?;

        // Wait on command
        let res = runner.wait()?;

        info!("Run complete '{}'", runner.cmd_line);

        Ok(res)
    }

    /// Spawn command to run in the background.
    ///
    /// Consumes the [`CmdBuilder`].
    pub fn spawn(self) -> Result<CmdRunner> {
        // Create temp file for stdout
        let (stdout_file, stdout_path) = NamedTempFile::new()?
            .keep()
            .map_err(|_| Error::StdoutTempfile)?;
        let stdout_path = String::from(stdout_path.to_str().ok_or(Error::StdoutTempfile)?);

        // Build expression to spawn
        let split = shellwords::split(&self.cmd_line)?;
        let mut expr = cmd(&split[0], &split[1..]);
        expr = expr.stdout_file(stdout_file);
        expr = expr.unchecked();
        for (key, val) in ENVIRONMENT_VARIABLES {
            expr = expr.env(key, val);
        }
        if let Some(bytes) = self.stdin {
            expr = expr.stdin_bytes(bytes);
        }

        // Spawn
        let handle = expr.start()?;
        info!(
            "Spawned '{}' (stdout path: '{}')",
            self.cmd_line, stdout_path
        );

        Ok(CmdRunner {
            cmd_line: self.cmd_line,
            timeout_ms: self.timeout_ms,
            handle,
            stdout_path,
        })
    }

    // Helper function
    fn stdout_modified(path: &str) -> Option<SystemTime> {
        fs::metadata(path).ok().and_then(|x| x.modified().ok())
    }

    // Helper function
    fn stdout_contents(path: &str) -> Result<String> {
        let mut output = String::new();
        let mut f = File::open(path)?;
        f.read_to_string(&mut output)?;
        Ok(output)
    }
}

pub struct CmdRunner {
    cmd_line: String,
    timeout_ms: u32,
    handle: Handle,
    stdout_path: String,
}

impl std::fmt::Debug for CmdRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CmdRunner")
            .field("cmd_line", &self.cmd_line)
            .field("timeout_ms", &self.timeout_ms)
            .field("stdout_path", &self.stdout_path)
            .finish()
    }
}

impl Drop for CmdRunner {
    /// Cleanup spwaned process.
    ///
    /// On unix, send SIGINT (Ctrl-C) to process to allow it the possibility
    /// of shutting itself down and saving its llvm coverage data.
    ///
    /// On non-unix, kill process.
    fn drop(&mut self) {
        if cfg!(unix) {
            let _ = self.handle.send_signal(libc::SIGINT);
        } else {
            let _ = self.handle.kill();
        }
    }
}

impl CmdRunner {
    /// Wait until a given regular expression matches the command's stdout.
    ///
    /// May timeout.
    ///
    /// If the regular expression is not found in stdout and the command has
    /// exited, will return Error::CmdExited.
    ///
    /// Function uses [`regex::Regex::captures()`] to find the capture groups
    /// corresponding to the leftmost-first match in stdout of the given regular
    /// expression. Capture groups are returned as `String`s.
    /// See [`regex::Regex::captures()`] for more information.
    pub fn match_stdout(&self, regex: &str) -> Result<Vec<Option<String>>> {
        let regex_obj = Regex::new(regex)?;

        info!(
            "Waiting to match regex on stdout (regex: '{regex}', cmd: '{}')",
            self.cmd_line
        );

        let mut modified_prev = None;

        for _ in 1..=(self.timeout_ms / POLL_MS) {
            // Get stdout contents from file and regex it
            let stdout = CmdBuilder::stdout_contents(&self.stdout_path)?;

            // Log tail of stdout, but not if we think it has not changed since the last time.
            let modified = CmdBuilder::stdout_modified(&self.stdout_path);
            if !stdout.is_empty() {
                if modified_prev.is_none() || modified_prev != modified {
                    info!(
                        "stdout tail (regex: '{}', cmd: '{}')...",
                        regex, self.cmd_line
                    );
                    let lines = stdout.as_str().lines().collect::<Vec<_>>();
                    for l in &lines[lines.len().saturating_sub(10)..] {
                        info!("  {l}");
                    }
                }
                modified_prev = modified;
            }

            // Try to match regex
            if let Some(captures) = regex_obj.captures(&stdout) {
                let mut v = Vec::new();
                for cap in captures.iter() {
                    let cap_string = cap.map(|tmp| String::from(tmp.as_str()));
                    v.push(cap_string);
                }
                return Ok(v);
            }

            // Bail if command has already exited
            if self.handle.try_wait()?.is_some() {
                return Err(Error::ExitedCmd);
            }

            // Sleep till next poll
            sleep(Duration::from_millis(POLL_MS as u64));
        }
        Err(Error::Timeout)
    }

    /// Wait for command to complete.
    /// May timeout.
    /// On success, returns command's exit code and stdout.
    pub fn wait(&self) -> Result<(Option<i32>, String)> {
        info!("Waiting for command (cmd: '{}')", self.cmd_line);
        for _ in 1..=(self.timeout_ms / POLL_MS) {
            if let Some(result) = self.handle.try_wait()? {
                info!(
                    "Command completed (cmd: '{}', exitcode: {:?})",
                    self.cmd_line,
                    result.status.code()
                );

                // Return exitcode, stdout contents
                let stdout = CmdBuilder::stdout_contents(&self.stdout_path)?;
                return Ok((result.status.code(), stdout));
            }
            sleep(Duration::from_millis(POLL_MS as u64));
        }
        Err(Error::Timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_not_found() {
        let res = CmdBuilder::new("iDoNotExist arg1 arg2").run();
        assert!(res.is_err());
    }

    #[test]
    fn curl_http_ockam() {
        let (exitcode, stdout) = CmdBuilder::new("curl -s -L http://ockam.io/")
            .run()
            .unwrap();
        assert_eq!(Some(0), exitcode);
        assert!(stdout.to_lowercase().contains("<html"));
    }

    #[test]
    fn bash_exitcode() {
        let (exitcode, stdout) = CmdBuilder::new("bash")
            .set_stdin("sleep 1; echo \"failed\"; exit 99")
            .run()
            .unwrap();
        assert_eq!(Some(99), exitcode);
        assert!(stdout.to_lowercase().contains("failed"));
    }

    #[test]
    fn bash_echo_sleep_match_on_stdout() {
        // Spawn to run for a few seconds
        let alice = CmdBuilder::new("bash")
            .set_stdin("echo \"Start: `date +%s`\"; sleep 3; echo \"End\"")
            .spawn()
            .unwrap();

        // Match on stdout, expect near start
        let tmp = alice
            .match_stdout(r"(?im)^Start: (\d+)")
            .unwrap()
            .swap_remove(1)
            .unwrap();
        assert!(tmp.chars().all(char::is_numeric));

        // Wait for completion
        let (exitcode, stdout) = alice.wait().unwrap();
        assert_eq!(Some(0), exitcode);
        assert!(stdout.contains("End"));
    }

    #[test]
    fn bash_echo_sleep_concurrent_commands() {
        // Spawn a command
        let alice = CmdBuilder::new("bash")
            .set_stdin("sleep 3; echo goodbye")
            .spawn()
            .unwrap();

        // Run another command
        let (exitcode, stdout) = CmdBuilder::new("echo hello").run().unwrap();
        assert_eq!(Some(0), exitcode);
        assert_eq!("hello", stdout.trim()); // Trim line endings

        // Wait on spawned
        let (exitcode, stdout) = alice.wait().unwrap();
        assert_eq!(Some(0), exitcode);
        assert!(stdout.contains("goodbye"));
    }

    #[test]
    fn bash_echo_sleep_concurrent_commands_donotwaitforspawned() {
        // Spawn a command
        let _alice = CmdBuilder::new("bash")
            .set_stdin("sleep 3; echo goodbye")
            .spawn()
            .unwrap();

        // Run another command
        let (exitcode, stdout) = CmdBuilder::new("echo hello").run().unwrap();
        assert_eq!(Some(0), exitcode);
        assert_eq!("hello", stdout.trim()); // Trim line endings

        // Don't wait on spawned command. It should get killed when dropped.
    }
}
