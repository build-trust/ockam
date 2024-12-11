mod server;
mod tls;

use clap::{Args, Parser, Subcommand};
use rustls::pki_types::ServerName;
use server::ServerMode;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tls::NoTlsValidation;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_rustls::rustls;

const DEFAULT_BUFFER_SIZE: &str = "65536";
const DEFAULT_BUFFER_POOL: &str = "4";

#[derive(Debug, Args, Clone)]
struct EchoCommand {
    bind_port: u16,
    #[arg(long, default_value = "false")]
    tls: bool,
    #[arg(long, default_value = DEFAULT_BUFFER_SIZE, env = "BUFFER_SIZE")]
    buffer_size: usize,
    #[arg(long, default_value = DEFAULT_BUFFER_POOL, env = "BUFFER_POOL")]
    buffer_pool: u8,
}

#[derive(Debug, Args, Clone)]
struct NullCommand {
    bind_port: u16,
    #[arg(long, default_value = "false")]
    tls: bool,
    #[arg(long, default_value = DEFAULT_BUFFER_SIZE, env = "BUFFER_SIZE")]
    buffer_size: usize,
}

#[derive(Debug, Args, Clone)]
struct LatencyCommand {
    address: SocketAddr,
    #[arg(long, default_value = "10")]
    count: u32,
    #[arg(long, default_value = "false")]
    tls: bool,
}

#[derive(Debug, Args, Clone)]
struct FloodCommand {
    address: SocketAddr,
    #[arg(long, default_value = "0")]
    max: u32,
}

#[derive(Debug, Args, Clone)]
struct ThroughputCommand {
    address: SocketAddr,
    #[arg(long, default_value = "false")]
    tls: bool,
    #[arg(long, default_value = DEFAULT_BUFFER_SIZE, env = "BUFFER_SIZE")]
    buffer_size: usize,
}

#[derive(Subcommand, Debug, Clone)]
enum Action {
    /// Measure the latency of a TCP connection
    Latency(LatencyCommand),
    /// Flood a TCP port with connections until it fails
    Flood(FloodCommand),
    /// Measure the throughput of TCP connection.
    /// Use it with `Null` or `Echo` servers.
    Throughput(ThroughputCommand),
    /// Run a TCP echo server
    Echo(EchoCommand),
    /// Run a TCP server, discards all incoming data
    Null(NullCommand),
}

#[derive(Debug, Parser, Clone)]
#[command(name = "tcp-tester")]
struct Main {
    /// Action to perform
    #[command(subcommand)]
    action: Action,
}

#[tokio::main]
async fn main() {
    let main: Main = Main::parse();
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    match main.action {
        Action::Echo(cmd) => {
            server::start_server(
                cmd.bind_port,
                cmd.tls,
                ServerMode::Echo(cmd.buffer_size, cmd.buffer_pool),
            )
            .await
        }
        Action::Null(cmd) => {
            server::start_server(cmd.bind_port, cmd.tls, ServerMode::Null(cmd.buffer_size)).await
        }
        Action::Latency(cmd) => latency(cmd).await,
        Action::Flood(cmd) => flood(cmd).await,
        Action::Throughput(cmd) => throughput(cmd).await,
    }
}

async fn throughput(command: ThroughputCommand) {
    let connector = if command.tls {
        let config = Arc::new(
            rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoTlsValidation))
                .with_no_client_auth(),
        );
        Some(tokio_rustls::TlsConnector::from(config))
    } else {
        None
    };

    let stream = tokio::net::TcpStream::connect(command.address)
        .await
        .unwrap();
    stream.set_nodelay(true).unwrap();
    let mut stream_write: Box<dyn AsyncWrite + Unpin + Send>;
    let mut stream_read: Box<dyn AsyncRead + Unpin + Send>;

    if let Some(connector) = &connector {
        let result = connector
            .connect(ServerName::IpAddress(command.address.ip().into()), stream)
            .await;
        match result {
            Ok(stream) => {
                let (r, w) = tokio::io::split(stream);
                stream_read = Box::new(r);
                stream_write = Box::new(w);
            }
            Err(error) => {
                println!("Failed to connect: {:?}", error);
                return;
            }
        }
    } else {
        let (r, w) = tokio::io::split(stream);
        stream_read = Box::new(r);
        stream_write = Box::new(w);
    }

    // read as much as possible and discard the result
    tokio::spawn(async move {
        let mut incoming_buffer = vec![0u8; command.buffer_size];
        loop {
            let result = stream_read.read(&mut incoming_buffer).await;
            if let Err(err) = result {
                println!("Failed to read: {:?}", err);
                return;
            }
        }
    });

    // read and report the speed
    let outgoing_buffer = vec![0u8; command.buffer_size];
    let mut total_bytes = 0;
    let mut iterations = 0;

    let mut start = Instant::now();
    loop {
        let result = stream_write.write_all(&outgoing_buffer).await;
        iterations += 1;
        match result {
            Ok(()) => {
                total_bytes += outgoing_buffer.len();
            }
            Err(err) => {
                println!("Failed to write: {:?}", err);
                return;
            }
        }

        if iterations > 10_000 {
            iterations = 0;
            let elapsed = start.elapsed();
            let seconds = elapsed.as_secs();
            if seconds >= 1 {
                println!(
                    "Outgoing Throughput: {:.2} Gbps",
                    (total_bytes as f32 / elapsed.as_secs_f32()) * 8.0 / 1_000_000_000.0
                );
                start = Instant::now();
                total_bytes = 0;
            }
        }
    }
}

async fn latency(command: LatencyCommand) {
    let mut first_rtt_measurements = Vec::new();
    let mut second_rtt_measurements = Vec::new();
    let mut incoming_buffer = [0u8; 5];
    let tls_connector = if command.tls {
        let config = Arc::new(
            rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoTlsValidation))
                .with_no_client_auth(),
        );
        Some(tokio_rustls::TlsConnector::from(config))
    } else {
        None
    };

    for _ in 0..command.count {
        let result = tokio::net::TcpStream::connect(command.address).await;
        match result {
            Ok(stream) => {
                stream.set_nodelay(true).unwrap();

                let mut stream_write: Box<dyn AsyncWrite + Unpin>;
                let mut stream_read: Box<dyn AsyncRead + Unpin>;

                let start = Instant::now();

                if let Some(tls_connector) = &tls_connector {
                    let result = tls_connector
                        .connect(ServerName::IpAddress(command.address.ip().into()), stream)
                        .await;
                    match result {
                        Ok(stream) => {
                            let (r, w) = tokio::io::split(stream);
                            stream_read = Box::new(r);
                            stream_write = Box::new(w);
                        }
                        Err(error) => {
                            println!("Failed to connect: {:?}", error);
                            continue;
                        }
                    }
                } else {
                    let (r, w) = tokio::io::split(stream);
                    stream_read = Box::new(r);
                    stream_write = Box::new(w);
                }

                let result = stream_write.write_all("hello".as_bytes()).await;
                if let Err(err) = result {
                    println!("Failed to write: {:?}", err);
                    continue;
                }
                let result = stream_read.read_exact(&mut incoming_buffer).await;
                if let Err(err) = result {
                    println!("Failed to read: {:?}", err);
                    continue;
                }
                let first_rtt = start.elapsed();
                first_rtt_measurements.push(first_rtt);

                let result = stream_write.write_all("hello".as_bytes()).await;
                if let Err(err) = result {
                    println!("Failed to write: {:?}", err);
                    continue;
                }
                let result = stream_read.read_exact(&mut incoming_buffer).await;
                if let Err(err) = result {
                    println!("Failed to read: {:?}", err);
                    continue;
                }
                let second_rtt = start.elapsed() - first_rtt;
                second_rtt_measurements.push(second_rtt);
                println!(
                    "Connection + First RTT: {}ms {}us, Second RTT: {}ms {}us",
                    first_rtt.as_millis(),
                    first_rtt.as_micros(),
                    second_rtt.as_millis(),
                    second_rtt.as_micros()
                );
            }
            Err(err) => {
                println!("Failed to connect: {:?}", err);
            }
        }
    }

    let total: std::time::Duration = first_rtt_measurements.iter().sum();
    let first_average = total / command.count;
    let total: std::time::Duration = second_rtt_measurements.iter().sum();
    let second_average = total / command.count;
    println!(
        "Average - Connection + First RTT: {}ms {}us, Second RTT: {}ms {}us",
        first_average.as_millis(),
        first_average.as_micros(),
        second_average.as_millis(),
        second_average.as_micros(),
    );
}

async fn flood(command: FloodCommand) {
    let mut connections = Vec::new();
    let mut counter = 0;
    loop {
        let result = tokio::net::TcpStream::connect(command.address).await;
        match result {
            Ok(stream) => {
                stream.set_nodelay(true).unwrap();
                connections.push(stream);
                counter += 1;
                if command.max > 0 && counter >= command.max {
                    break;
                }
            }
            Err(err) => {
                println!("Failed to connect: {:?}", err);
                break;
            }
        }
    }

    println!("Flooded {} connections", connections.len());
    println!("Pinging each...");

    let mut failed = 0;
    let mut incoming_buffer = [0u8; 5];
    for mut stream in connections {
        let result = stream.write_all("hello".as_bytes()).await;
        if let Err(err) = result {
            println!("Failed to write: {:?}", err);
            failed += 1;
            continue;
        }
        let result = stream.read_exact(&mut incoming_buffer).await;
        if let Err(err) = result {
            println!("Failed to read: {:?}", err);
            failed += 1;
        }
    }

    if failed == 0 {
        println!("All connections succeeded");
    } else {
        println!("{} connections failed", failed);
    }
}
