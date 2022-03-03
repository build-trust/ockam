//! Note: Much of this should be moved into either the CLI tool, a dedicated
//! crate, or really just somewhere else (it's how it is because of
//! ✨deadlines✨).
use anyhow::{anyhow, Error, Result};
use clap::Parser;
#[macro_use]
extern crate tracing;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

pub fn init_logging(verbose: bool) {
    let filter = EnvFilter::try_from_env("OCKAM_LOG").unwrap_or_else(|_| {
        EnvFilter::default().add_directive(if verbose { LevelFilter::DEBUG } else { LevelFilter::INFO }.into())
    });
    if fmt().with_env_filter(filter).try_init().is_err() {
        warn!("Failed to initialise logging.");
    }
}

pub fn init_data_dir(dir: &Option<std::path::PathBuf>) -> std::io::Result<()> {
    if let Some(p) = dir {
        tracing::info!(
            "`--data_dir={:?}` flag passed, and vault will be persisted at that location",
            p,
        );
        tracing::info!("Creating directories at {:?} for vault persistence", p);
        std::fs::create_dir_all(dir)?;
        Ok(())
    } else {
        tracing::info!("No `--data_dir` flag passed, vault persistence is not enabled");
    }
}

// Uses the following algorithm.
// - if environment variable `$OCKAM_DIR` is set, use `$OCKAM_DIR`.
// - if exists, and `$HOME/.config`
// - if environment variable `$HOME` exists and if directory `$HOME/.ockam`
pub fn default_ockam_dir() -> std::path::PathBuf {
    // TODO: I'd like to use $OCKAM_HOME for this, but currently we use it for
    // the repo root. We should switch the repo root to $OCKAM_REPOSITORY or
    // something instead (or even better: remove the need for it)
    if let Some(d) = std::env::var_os("OCKAM_DIR") {
        return Ok(d.into());
    }
    if cfg!(unix) {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            anyhow!("Failed to locate ockam directory (this can be resolved by `OCKAM_DIR` in the environment)")
        })?;
        if cfg!(target_os = "macos") {
            if home_dir.join(".config").exists() {
                return Ok(home_dir.join(".config").join("ockam"));
            }
        }
    }
}

fn message(verbose: bool, e: &dyn std::error::Error) -> impl core::fmt::Display {
    if verbose {
        format_args!("{:?}", e)
    } else {
        format_args!("{}", e)
    }
}
pub fn exit_with_result(verbose: bool, result: Result<anyhow::Result<()>, ockam::Error>) -> ! {
    let res = match result {
        Err(e) => Err(anyhow::anyhow!(e)),
        Ok(Err(e)) => Err(e),
        Ok(Ok(())) => Ok(()),
    };

    if let Err(e) = res {
        tracing::error!("Error during execution: {}", message(&e));
        for cause in e.chain().skip(1) {
            tracing::error!("- caused by: {}", message(cause));
        }
        std::process::exit(1);
    } else {
        std::process::exit(0);
    }
}
