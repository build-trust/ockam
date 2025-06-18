use crate::util::parsers::hostname_parser;
use crate::zone::ctrlc::ZoneCtrlcHandler;
use crate::zone::watcher::DirectoryWatcher;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use indicatif::ProgressBar;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam::transport::SchemeHostnamePort;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_node::Context;
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Completer, Editor, Helper, Highlighter, Hinter};
use std::cmp::PartialEq;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::debug;

const LONG_ABOUT: &str = include_str!("./static/repl/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/repl/after_long_help.txt");

/// Open a repl to a local server
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ReplCommand {
    /// Network address where your repl server is listening to.
    #[arg(long, id = "SOCKET_ADDRESS", value_parser = hostname_parser)]
    pub to: SchemeHostnamePort,
}

#[async_trait]
impl Command for ReplCommand {
    const NAME: &'static str = "zone repl";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        self.run_impl(opts, ctx).await
    }
}

impl ReplCommand {
    pub async fn run_impl(self, opts: CommandGlobalOpts, _ctx: &Context) -> Result<()> {
        // Open the repl to the given address, without an inlet
        self.open_repl(&opts).await
    }

    pub async fn open_repl(&self, opts: &CommandGlobalOpts) -> miette::Result<()> {
        if !opts.terminal.is_tty() {
            return Ok(());
        }

        use tokio::net::TcpStream;
        use tokio::time::sleep;

        let server_address = self.to.clone();

        let (next_tx, next_rx) = tokio::sync::mpsc::channel(16);
        let (lines_tx, lines_rx) = tokio::sync::mpsc::channel(16);
        let mut stdin = RustylineHandle { lines_rx, next_tx };
        let stdin_handle =
            tokio_util::task::AbortOnDropHandle::new(tokio::task::spawn(async move {
                if let Err(e) = RustylineHandler::run_repl(next_rx, lines_tx, None).await {
                    eprintln!("{:?}", e);
                }
            }));

        // Start the interactive repl
        let mut initial_message_spinner = opts.terminal.spinner();
        if let Some(spinner) = &initial_message_spinner {
            spinner.set_message("Waiting for repl to be ready...");
        }
        let mut _initial_message_spinner = initial_message_spinner.clone();
        let _opts = opts.clone();
        let repl_handle = tokio_util::task::AbortOnDropHandle::new(tokio::spawn(async move {
            async fn connect_to_inlet(addr: &SchemeHostnamePort) -> miette::Result<TcpStream> {
                const MAX_RETRIES: u32 = 120; // Try for about 60 seconds (120 * 500ms)
                let duration = Duration::from_millis(500);
                let mut retries = 0;

                while retries < MAX_RETRIES {
                    match TcpStream::connect(addr.hostname_port().to_string()).await {
                        Ok(s) => return Ok(s),
                        Err(_) => {
                            retries += 1;
                            sleep(duration).await;
                        }
                    }
                }
                Err(miette!("Failed to connect to the TCP Inlet"))
            }

            let mut read_header_buffer = String::new();
            let mut read_body_buffer = Vec::new();
            'repl: loop {
                // 1: Try to connect to the inlet
                let stream = match connect_to_inlet(&server_address).await {
                    Ok(s) => s,
                    Err(_e) => {
                        sleep(Duration::from_secs(1)).await;
                        continue 'repl;
                    }
                };

                let (reader, mut writer) = stream.into_split();
                let mut reader = BufReader::new(reader);

                // 2: Try to read initial message
                match Self::wait_until_server_is_ready(
                    &mut reader,
                    &mut read_header_buffer,
                    &mut read_body_buffer,
                )
                .await
                {
                    Ok(res) => {
                        // Successfully got initial message, continue to repl
                        if let Some(spinner) = _initial_message_spinner.take() {
                            spinner.finish_and_clear();
                        }
                        _opts.terminal.write(res)?;
                    }
                    Err(_e) => {
                        if _initial_message_spinner.is_none() {
                            _initial_message_spinner = _opts.terminal.spinner();
                        }
                        if let Some(spinner) = &_initial_message_spinner {
                            spinner.set_message("Waiting for repl to be ready...");
                        }
                        sleep(Duration::from_secs(1)).await;
                        continue 'repl;
                    }
                }

                // 3: Start the repl loop for this connection
                'connection: loop {
                    // Get user input
                    let user_input = match stdin.read_line().await {
                        Some(i) => i,
                        None => {
                            // stdin channel closed, exit repl
                            break 'repl;
                        }
                    };

                    if user_input.trim().is_empty() {
                        continue 'connection;
                    }

                    if is_reconnect_sequence(&user_input) {
                        break 'connection;
                    }

                    if is_quit_sequence(&user_input) {
                        let _ = writer.write_all(user_input.as_bytes()).await;
                        let _ = writer.flush().await;
                        break 'repl;
                    }

                    // Send input to server
                    if let Err(e) = writer.write_all(user_input.as_bytes()).await {
                        debug!("failed to write to server: {:?}", e);
                        _opts
                            .terminal
                            .write_line("Failed to write to server. Reconnecting...")?;
                        break 'connection;
                    }
                    let _ = writer.flush().await;

                    // Wait for server response and print it
                    if let Err(e) = Self::process_server_response(
                        &_opts,
                        &mut reader,
                        &mut read_header_buffer,
                        &mut read_body_buffer,
                    )
                    .await
                    {
                        _opts.terminal.write_line(format!(
                            "Failed to read from server ({e}). Reconnecting...",
                        ))?;
                        break 'connection;
                    }
                }

                // If we break from the connection loop, we'll go back to connecting
                sleep(Duration::from_secs(1)).await;
            }

            Ok::<(), miette::Error>(())
        }));

        tokio::select! {
            _ = ZoneCtrlcHandler::wait_for_message() => {
                ZoneCtrlcHandler::send();
            },
            _ = DirectoryWatcher::wait_for_message() => {},
            res = stdin_handle => {
                res.into_diagnostic()?;
                ZoneCtrlcHandler::send();
            },
            res = repl_handle => {
                res.into_diagnostic()??;
                ZoneCtrlcHandler::send();
            },
        }

        if let Some(spinner) = initial_message_spinner.take() {
            spinner.finish_and_clear();
        }

        Ok(())
    }

    async fn wait_until_server_is_ready(
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        read_header_buffer: &mut String,
        read_body_buffer: &mut Vec<u8>,
    ) -> std::result::Result<String, String> {
        match Self::read_server_response(
            reader,
            read_header_buffer,
            read_body_buffer,
            Duration::from_secs(5),
        )
        .await
        {
            ReadStatus::Success(res) => Ok(res),
            status => Err(status.to_string()),
        }
    }

    async fn process_server_response(
        opts: &CommandGlobalOpts,
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        read_header_buffer: &mut String,
        read_body_buffer: &mut Vec<u8>,
    ) -> std::result::Result<(), String> {
        let mut finished = false;
        let mut timeout = Duration::from_secs(30);
        let mut spinner: Option<Option<ProgressBar>> = None;
        let mut result = Ok(());
        opts.terminal.write("\n").map_err(|e| e.to_string())?;
        while !finished {
            match Self::read_server_response(reader, read_header_buffer, read_body_buffer, timeout)
                .await
            {
                ReadStatus::Success(res) => {
                    opts.terminal.write(&res).map_err(|e| e.to_string())?;
                }
                ReadStatus::EndOfStream => {
                    opts.terminal.write("\n\n").map_err(|e| e.to_string())?;
                    finished = true;
                }
                ReadStatus::Timeout => {
                    // Short timeout reached, notify but keep waiting
                    spinner = Some(opts.terminal.spinner());
                    if let Some(Some(spinner)) = &spinner {
                        spinner.set_message("Waiting for repl response...");
                    }
                    timeout = Duration::from_secs(5 * 60);
                }
                status => {
                    finished = true;
                    result = Err(status.to_string());
                }
            };
            read_body_buffer.clear();
        }
        if let Some(Some(spinner)) = &spinner {
            spinner.finish_and_clear();
        }
        result
    }

    async fn read_server_response(
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        read_header_buffer: &mut String,
        read_body_buffer: &mut Vec<u8>,
        timeout_duration: Duration,
    ) -> ReadStatus {
        // Use timeout only for the header read
        read_header_buffer.clear();
        match timeout(timeout_duration, reader.read_line(read_header_buffer)).await {
            Ok(Ok(0)) => ReadStatus::ConnectionClosed,
            Ok(Ok(_n)) => {
                // Process the header as a line containing the body length
                let body_len: usize = match read_header_buffer.trim().parse() {
                    Ok(v) => v,
                    Err(e) => {
                        debug!("failed to read header length: {e}");
                        return ReadStatus::Error("header parsing".to_string());
                    }
                };

                if body_len == 0 {
                    ReadStatus::EndOfStream
                } else {
                    // Process the body
                    read_body_buffer.resize(body_len, 0);
                    if let Err(e) = reader.read_exact(read_body_buffer).await {
                        debug!("failed to read body: {e}");
                        return ReadStatus::Error("body parsing".to_string());
                    };
                    let body = String::from_utf8_lossy(read_body_buffer).into_owned();
                    ReadStatus::Success(body)
                }
            }
            Ok(Err(e)) => {
                debug!("failed to read from server: {e}");
                ReadStatus::Error("server".to_string())
            }
            Err(_) => ReadStatus::Timeout,
        }
    }
}

/// Models the possible outcomes when reading server responses
#[derive(Debug)]
enum ReadStatus {
    /// Successfully read data
    Success(String),

    /// No more data to read
    EndOfStream,

    /// Server connection was closed
    ConnectionClosed,

    /// Error occurred during reading
    Error(String),

    /// Initial read operation timed out
    Timeout,
}

impl Display for ReadStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadStatus::Success(_) => write!(f, "success"),
            ReadStatus::EndOfStream => write!(f, "end of stream"),
            ReadStatus::ConnectionClosed => write!(f, "connection closed"),
            ReadStatus::Error(e) => write!(f, "{e}"),
            ReadStatus::Timeout => write!(f, "timeout"),
        }
    }
}

struct RustylineHandler {}

struct RustylineHandle {
    lines_rx: tokio::sync::mpsc::Receiver<String>,
    next_tx: tokio::sync::mpsc::Sender<()>,
}

impl RustylineHandle {
    async fn read_line(&mut self) -> Option<String> {
        match self.next_tx.send(()).await {
            Ok(_) => self.lines_rx.recv().await.map(|line| line + "\n"),
            Err(_) => {
                // Channel closed, exit repl
                None
            }
        }
    }
}

#[derive(Hinter, Completer, Highlighter)]
struct MultiLineValidatorHelper {}
impl Validator for MultiLineValidatorHelper {
    fn validate(
        &self,
        ctx: &mut ValidationContext,
    ) -> std::result::Result<ValidationResult, ReadlineError> {
        let input = ctx.input();
        // We don't get the final \n here, so the first time will be just the marker,
        // then on later calls inside this multiline input, it does have the newline between the
        // marker and whatever the next line is.
        if input.starts_with("\"\"\"\n") || input.eq("\"\"\"") {
            return if input.ends_with("\n\"\"\"") || is_quit_sequence(input) {
                Ok(ValidationResult::Valid(None))
            } else {
                Ok(ValidationResult::Incomplete)
            };
        }
        Ok(ValidationResult::Valid(None))
    }

    fn validate_while_typing(&self) -> bool {
        false
    }
}

impl Helper for MultiLineValidatorHelper {}

impl RustylineHandler {
    async fn run_repl(
        mut next_rx: tokio::sync::mpsc::Receiver<()>,
        lines_tx: tokio::sync::mpsc::Sender<String>,
        history_file_path: Option<PathBuf>,
    ) -> Result<()> {
        let history_file_path = Self::get_history_file_path(history_file_path)?;
        let mut rl = Editor::new()
            .into_diagnostic()
            .wrap_err("Failed to initialize repl")?;
        rl.set_max_history_size(50).into_diagnostic()?;
        let helper = MultiLineValidatorHelper {};
        rl.set_helper(Some(helper));
        rl.load_history(&history_file_path).into_diagnostic()?;

        loop {
            // If we can't read from the channel, exit
            if next_rx.recv().await.is_none() {
                break;
            };

            // We run rustyline on their own blocking thread. The thread waits for an input line to be requested,
            // then reads it, and delivers it. While waiting for agent response, there is no line requested,
            // so readline is not active. If the thread terminates, the entire repl exits.
            let readline = tokio::task::block_in_place(|| rl.readline("> "));
            match readline {
                Ok(line) => {
                    if is_quit_sequence(&line) {
                        break;
                    }
                    rl.add_history_entry(line.as_str()).into_diagnostic()?;
                    if lines_tx.send(line).await.is_err() {
                        break;
                    }
                    let _ = rl.append_history(&history_file_path);
                }
                Err(err) => {
                    debug!("Failed to read line: {:?}", err);
                    break;
                }
            }
        }
        Ok(())
    }

    fn get_history_file_path(history_file_path: Option<PathBuf>) -> Result<PathBuf> {
        let create_history_file = |path: &PathBuf| -> Result<()> {
            if !path.exists() {
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent)
                            .into_diagnostic()
                            .wrap_err("Failed to create history directory for repl")?;
                    }
                }
                std::fs::File::create(path)
                    .into_diagnostic()
                    .wrap_err("Failed to create history file for repl")?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let permissions = std::fs::Permissions::from_mode(0o600);
                    std::fs::set_permissions(path, permissions)
                        .into_diagnostic()
                        .wrap_err("Failed to set permissions on history file")?;
                }
            }
            Ok(())
        };
        match history_file_path {
            Some(path) => {
                if !path.exists() {
                    create_history_file(&path)?;
                }
                Ok(path)
            }
            None => {
                let default_path = std::env::current_dir()
                    .into_diagnostic()
                    .wrap_err("Failed to get current directory")?
                    .join(".repl/history.txt");
                create_history_file(&default_path)?;
                Ok(default_path)
            }
        }
    }
}

fn is_reconnect_sequence(input: &str) -> bool {
    let i = input.trim();
    let seq = [":reconnect", ":r"];
    seq.contains(&i)
}

fn is_quit_sequence(input: &str) -> bool {
    let i = input.trim();
    let seq = [":quit", ":q"];
    if seq.contains(&i) {
        return true;
    }
    let last_line = i.lines().last().unwrap_or("").trim();
    seq.contains(&last_line)
}

#[allow(dead_code)]
pub(super) mod dummy_server {
    use super::*;

    impl ReplCommand {
        /// Start a TCP server that echoes back any input
        pub(super) async fn dummy_inlet(
            &self,
            opts: &CommandGlobalOpts,
            inlet_address: SchemeHostnamePort,
        ) -> miette::Result<(Option<JoinHandle<Result<()>>>, Vec<JoinHandle<Result<()>>>)> {
            use tokio::net::TcpListener;

            let addr = inlet_address.hostname_port().to_string();
            let inlet_handle = tokio::spawn(async move {
                let listener = TcpListener::bind(&addr)
                    .await
                    .into_diagnostic()
                    .wrap_err(format!("Failed to bind to {}", addr))?;
                loop {
                    let (socket, _) = listener
                        .accept()
                        .await
                        .into_diagnostic()
                        .wrap_err("Failed to accept connection")?;
                    tokio::spawn(async move {
                        if let Err(err) = Self::handle_connection(socket).await {
                            eprintln!("Connection error: {:?}", err);
                        }
                    });
                }
            });

            opts.terminal.write_line(fmt_ok!(
                "Dummy echo inlet started on {}",
                color_primary(inlet_address.to_string())
            ))?;

            // Ok((Some(inlet_handle), vec![]))
            Ok((None, vec![inlet_handle]))
        }

        async fn handle_connection(socket: tokio::net::TcpStream) -> miette::Result<()> {
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);

            // Initial message
            let welcome =
                "Welcome to the dummy echo inlet. Type anything and it will be echoed back.\n";
            Self::send_formatted_response(&mut writer, welcome).await?;

            let mut buffer = String::new();

            loop {
                buffer.clear();
                match reader.read_line(&mut buffer).await {
                    Ok(0) => {
                        return Ok(());
                    }
                    Ok(_n) => {}
                    Err(e) => {
                        return Err(miette!("Failed to read from socket: {}", e));
                    }
                };

                if buffer.trim().is_empty() {
                    continue;
                }

                // Echo data back
                let message = format!("Echo: {}", buffer);
                Self::send_formatted_response(&mut writer, &message).await?;
            }
        }

        async fn send_formatted_response(
            writer: &mut tokio::net::tcp::OwnedWriteHalf,
            message: &str,
        ) -> miette::Result<()> {
            // First line contains just the length
            let header = format!("{}\n", message.len());
            writer
                .write_all(header.as_bytes())
                .await
                .into_diagnostic()
                .wrap_err("Failed to write header to socket")?;
            writer
                .write_all(message.as_bytes())
                .await
                .into_diagnostic()
                .wrap_err("Failed to write message to socket")?;
            writer
                .flush()
                .await
                .into_diagnostic()
                .wrap_err("Failed to flush message")?;
            Ok(())
        }
    }
}
