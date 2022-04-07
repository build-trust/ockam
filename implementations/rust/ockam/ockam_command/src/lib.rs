//! This library exists only to be used by the `ockam` CLI (in
//! `./bin/ockam.rs`).
//!
//! It should only publically expose a [single item](`run_main`) which should
//! called from `fn main()` of `bin/ockam.rs` as the only thing it does.

use self::args::*;
use anyhow::{Context, Result};
use clap::Parser;
use identity::load_identity;
use ockam::identity::IdentityIdentifier;
use std::collections::BTreeSet;
use storage::{ensure_identity_exists, get_ockam_dir};
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

pub(crate) type OckamVault = ockam::vault::VaultMutex<ockam_vault::SoftwareVault>;

pub(crate) mod args;
pub(crate) mod identity;
pub(crate) mod storage;
pub(crate) mod cmd {
    pub(crate) mod identity;
    pub(crate) mod inlet;
    pub(crate) mod outlet;
}

pub(crate) mod session {
    pub(crate) mod error;
    pub(crate) mod initiator;
    pub(crate) mod msg;
    pub(crate) mod responder;
}

// This should be this library's only public function.
pub fn run_main() {
    let args = CliArgs::parse();
    let verbose = args.verbose;
    init_logging(verbose);
    tracing::debug!("Parsed arguments (outlet) {:?}", args);
    // Note: We don't force all commands to start the node.
    match args.command {
        args::Command::CreateIdentity(arg) => node_subcommand(verbose > 0, arg, cmd::identity::run),
        args::Command::CreateInlet(arg) => node_subcommand(verbose > 0, arg, cmd::inlet::run),
        args::Command::CreateOutlet(arg) => node_subcommand(verbose > 0, arg, cmd::outlet::run),
        args::Command::AddTrustedIdentity(arg) => exit_with_result(verbose > 0, add_trusted(arg)),
        args::Command::PrintIdentity => exit_with_result(verbose > 0, print_identity()),
        args::Command::PrintPath => exit_with_result(verbose > 0, print_ockam_dir()),
    }
}

fn print_identity() -> anyhow::Result<()> {
    ensure_identity_exists(false)?;
    let dir = get_ockam_dir()?;
    let identity = load_identity(&dir.join("identity.json"))?;
    println!("{}", identity.id.key_id());
    Ok(())
}

fn print_ockam_dir() -> anyhow::Result<()> {
    match get_ockam_dir() {
        Ok(path) => {
            // We'd rather panic than print a lossy (and thus possibly wrong)
            // path. But `get_ockam_dir()` checks this.
            println!("{}", path.to_str().expect("bug in `get_ockam_dir`"));
            Ok(())
        }
        Err(e) => {
            eprintln!(
                "Failed to locate the ockam directory (or it was invalid). \
                Hint: try providing `$OCKAM_DIR` explicitly, or changing the \
                value of `$OCKAM_DIR` if it is already set in your environment.",
            );
            Err(e)
        }
    }
}

fn add_trusted(arg: AddTrustedIdentityOpts) -> anyhow::Result<()> {
    // Parse args before we start complaining about the directory.
    let to_trust = crate::identity::parse_identities(&arg.to_trust)?;
    ensure_identity_exists(false)?;
    let ockam_dir = get_ockam_dir()?;
    let trusted_file = ockam_dir.join("trusted");
    if to_trust.is_empty() && !arg.only {
        eprintln!(
            "No change to {} needed, no identities were \
            provided (and `--only` is not in use).",
            trusted_file.display(),
        );
        return Ok(());
    }
    let existing = if trusted_file.exists() && !arg.only {
        crate::identity::read_trusted_idents_from_file(&trusted_file)?
    } else {
        vec![]
    };
    let need = to_trust
        .into_iter()
        .filter(|id| !existing.contains(id))
        .collect::<Vec<_>>();
    if need.is_empty() && !arg.only {
        eprintln!(
            "No change to {} needed, all identities already \
            trusted (and `--only` is not in use).",
            trusted_file.display(),
        );
        return Ok(());
    }
    let all: Vec<_> = existing.iter().chain(need.iter()).cloned().collect();
    let all_dedup = all
        .iter()
        .cloned()
        .collect::<BTreeSet<IdentityIdentifier>>();
    // Keep user-provided order if no duplicates.
    let idents_to_write = if all_dedup.len() == all.len() {
        all
    } else {
        all_dedup.into_iter().collect::<Vec<_>>()
    };
    if idents_to_write == existing && !arg.only {
        eprintln!(
            "No change to {} needed. New and old trusted lists \
            would be identical (and `--only` is not in use).",
            trusted_file.display(),
        );
        return Ok(());
    }
    let strings_to_write = idents_to_write
        .into_iter()
        .map(|s| s.key_id().clone())
        .collect::<Vec<String>>();
    let new_contents = strings_to_write.join("\n");

    crate::storage::write(&trusted_file, new_contents.as_bytes()).with_context(|| {
        format!(
            "Writing updated list of trusted identities to {:?}",
            trusted_file
        )
    })?;
    eprintln!(
        "Wrote updated list to {}, containing {} identities.",
        trusted_file.display(),
        strings_to_write.len(),
    );
    Ok(())
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
            print_error_and_exit(verbose, e);
        }
    });
    if let Err(e) = res {
        eprintln!(
            "Ockam node failed at last minute. TODO: find out something \
            smarter to do if this happens: {:?}",
            e,
        );
    }
}

fn print_error_and_exit(v: bool, e: anyhow::Error) -> ! {
    tracing::trace!("Exiting with error {:?}", e);
    eprintln!("Error: {}", message(v, &e));
    for cause in e.chain().skip(1) {
        eprintln!("- caused by: {}", message(v, cause));
    }
    std::process::exit(1);
}

fn exit_with_result(verbose: bool, result: Result<()>) -> ! {
    if let Err(e) = result {
        print_error_and_exit(verbose, e);
    } else {
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

// Not really ideal, but fine for now.
fn init_logging(verbose: u8) {
    let ockam_crates = [
        "ockam",
        "ockam_node",
        "ockam_core",
        "ockam_command",
        "ockam_identity",
        "ockam_channel",
        "ockam_transport_tcp",
        "ockam_vault",
        "ockam_vault_sync_core",
    ];
    let builder = EnvFilter::builder();
    let filter = match std::env::var("OCKAM_LOG") {
        Ok(s) if !s.is_empty() => builder.with_env_var("OCKAM_LOG").from_env_lossy(),
        _ => match verbose {
            0 => builder
                .with_default_directive(LevelFilter::WARN.into())
                .parse_lossy(ockam_crates.map(|c| format!("{c}=info")).join(",")),
            1 => builder
                .with_default_directive(LevelFilter::INFO.into())
                .parse_lossy(""),
            2 => builder
                .with_default_directive(LevelFilter::DEBUG.into())
                .parse_lossy(""),
            _ => builder
                .with_default_directive(LevelFilter::TRACE.into())
                .parse_lossy(""),
        },
    };
    if fmt().with_env_filter(filter).try_init().is_err() {
        tracing::warn!("Failed to initialise logging.");
    }
}
