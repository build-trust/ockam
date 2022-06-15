use anyhow::Context;
use anyhow::Result;
use clap::Args;
use identity::load_identity;
use ockam::identity::IdentityTrait;
use ockam::{identity::IdentityIdentifier, NodeBuilder};
use std::collections::BTreeSet;
use storage::{ensure_identity_exists, get_ockam_dir};

pub mod identity;
pub mod storage;

pub mod cmd {
    // pub mod api;
    // pub mod start_node;
    pub mod identity;
    pub mod inlet;
    pub mod outlet;
}

pub mod session {
    pub mod error;
    pub mod initiator;
    pub mod msg;
    pub mod responder;
}

pub type OckamVault = ockam::vault::Vault;

pub async fn print_identity(_arg: (), mut ctx: ockam::Context) -> anyhow::Result<()> {
    ensure_identity_exists(false)?;
    let dir = get_ockam_dir()?;
    let identity = load_identity(&ctx, &dir).await?;
    let identifier = identity.identifier().await?;
    println!("{}", identifier.key_id());
    ctx.stop().await?;
    Ok(())
}

pub fn print_ockam_dir() -> anyhow::Result<()> {
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

#[derive(Clone, Debug, Args)]
pub struct AddTrustedIdentityOpts {
    /// Discard any identities currently in `~/.config/ockam/trusted`, and
    /// replace them with the ones provided by this command.
    #[clap(long)]
    pub only: bool,
    /// The identity to trust, or space/comma-separated list of identities.
    ///
    /// Some effort is taken to avoid writing the file when not necessary, and
    /// to avoid adding duplicates entries in the file. Note that
    pub to_trust: String,
}

pub fn add_trusted(arg: AddTrustedIdentityOpts) -> anyhow::Result<()> {
    // Parse args before we start complaining about the directory.
    let to_trust = crate::old::identity::parse_identities(&arg.to_trust)?;
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
        crate::old::identity::read_trusted_idents_from_file(&trusted_file)?
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

    crate::old::storage::write(&trusted_file, new_contents.as_bytes()).with_context(|| {
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

pub fn node_subcommand<A, F, Fut>(verbose: bool, arg: A, f: F)
where
    A: Send + Sync + 'static,
    F: FnOnce(A, ockam::Context) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let (ctx, mut executor) = NodeBuilder::without_access_control().no_logging().build();
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

pub(crate) fn print_error_and_exit(v: bool, e: anyhow::Error) -> ! {
    tracing::trace!("Exiting with error {:?}", e);
    eprintln!("Error: {}", message(v, &e));
    for cause in e.chain().skip(1) {
        eprintln!("- caused by: {}", message(v, cause));
    }
    std::process::exit(1);
}

pub(crate) fn exit_with_result(verbose: bool, result: Result<()>) -> ! {
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
