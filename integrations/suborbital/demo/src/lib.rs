#![allow(unused)]
use self::args::*;
use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

pub(crate) mod args;
pub(crate) mod identity;
pub(crate) mod storage;
pub(crate) mod vault;
pub(crate) mod cmd {
    pub(crate) mod identity;
    pub(crate) mod inlet;
    pub(crate) mod outlet;
}

// This should be this library's only public function.
pub fn run_main() {
    let args = Args::parse();
    let verbose = args.verbose;
    init_logging(verbose);
    tracing::debug!("Parsed arguments (outlet) {:?}", args);
    // Note: We don't force all commands to start the node.
    match args.command {
        args::Command::CreateIdentity(arg) => node_subcommand(verbose > 0, arg, cmd::identity::run),
        args::Command::CreateInlet(arg) => node_subcommand(verbose > 0, arg, cmd::inlet::run),
        args::Command::CreateOutlet(arg) => node_subcommand(verbose > 0, arg, cmd::outlet::run),
    }
}

fn node_subcommand<A, F, Fut>(verbose: bool, arg: A, f: F)
where
    A: Send + Sync + 'static,
    F: FnOnce(A, ockam::Context) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let (ctx, mut executor) = ockam::start_node();
    let res = executor.execute(async move {
        if let Err(e) = f(arg, ctx).await {
            eprintln!("Error during execution: {}", message(verbose, &e));
            for cause in e.chain().skip(1) {
                eprintln!("- caused by: {}", message(verbose, cause));
            }
            std::process::exit(1);
        }
    });
    if let Err(e) = res {
        eprintln!(
            "Ockam node failed at last minute. TODO: find out something smarter to do if this happens: {:?}",
            e
        );
    }
}

fn exit_with_result(verbose: bool, result: Result<()>) -> ! {
    if let Err(e) = result {
        eprintln!("Error during execution: {}", message(verbose, &e));
        for cause in e.chain().skip(1) {
            eprintln!("- caused by: {}", message(verbose, cause));
        }
        std::process::exit(1);
    } else {
        tracing::info!("Exiting (success)");
        std::process::exit(0);
    }
}

fn message(verbose: bool, e: impl std::fmt::Display + std::fmt::Debug) -> String {
    if verbose {
        format!("{:?}", e)
    } else {
        format!("{}", e)
    }
}

fn init_logging(verbose: u8) {
    let filter = EnvFilter::try_from_env("OCKAM_LOG")
        .unwrap_or_else(|_| {
            if verbose == 0 {
                EnvFilter::default().add_directive("ockam_node=warn".parse().unwrap())
            } else {
                EnvFilter::default()
            }
        })
        .add_directive(match verbose + 1 {
            0 => LevelFilter::WARN.into(),
            1 => LevelFilter::INFO.into(),
            2 => LevelFilter::DEBUG.into(),
            _ => LevelFilter::TRACE.into(),
        });
    if fmt().with_env_filter(filter).try_init().is_err() {
        tracing::warn!("Failed to initialise logging.");
    }
}
