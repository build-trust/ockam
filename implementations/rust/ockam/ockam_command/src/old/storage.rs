//! In theory this is the file that operates on the config dir itself, but this
//! is all a bit messy.
use std::path::PathBuf;

use anyhow::Context;
use ockam::access_control::IdentityIdAccessControl;

use ockam::identity::*;

fn config_home() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        // Use ~/.config/ockam on macOS too to avoid surprise differences
        // between dev/prod. By default, `dirs` wants to put it inside
        // ~/Library/Preferences which is clearly wrong for us anyway.
        dirs::home_dir()
            .map(|d| d.join(".config"))
            .or_else(dirs::config_dir)
    } else {
        dirs::config_dir()
    }
}

pub fn get_ockam_dir() -> anyhow::Result<PathBuf> {
    // TODO: ideally we'd use `$OCKAM_HOME` but all of our build tools assume
    // that's the repo...
    let ockam_dir = if let Some(s) = std::env::var_os("OCKAM_DIR") {
        let pb = PathBuf::from(s);
        if !pb.is_absolute() || pb.parent().is_none() {
            anyhow::bail!(
                "`OCKAM_DIR` must be an non-root absolute path, got: `{}`",
                pb.display()
            )
        }
        pb
    } else if let Some(cfg) = config_home() {
        cfg.join("ockam")
    } else {
        anyhow::bail!(
            "Failed to your OS configuration directory, and `OCKAM_DIR` is not provided. \
             This may be specified manually by setting `OCKAM_DIR` in your environment."
        );
    };
    if ockam_dir.to_str().is_none() {
        anyhow::bail!(
            "The ockam directory's path must be unambiguously \
            representable using UTF-8. Got ({ockam_dir:?})."
        );
    }
    Ok(ockam_dir)
}

pub fn ensure_identity_exists(expect_trusted: bool) -> anyhow::Result<()> {
    let dir = get_ockam_dir()?;
    if !dir.exists() {
        anyhow::bail!(
            "Failed to locate the ockam directory. Do you need to run `ockam create-identity`?"
        );
    }
    if !dir.join("identity.json").exists() {
        anyhow::bail!(
            "No identity has been initialized in the ockam directory at `{}`. \
            You may need to need to run `ockam create-identity`. If this directory \
            should not be used, you may use the `OCKAM_DIR` environment variable \
            instead.",
            dir.display(),
        );
    }
    if expect_trusted && !dir.join("trusted").exists() {
        eprintln!(
            "warning: The ockam directory does not have a list of trusted \
            identifiers at `{}/trusted`. This may indicate a configuration error",
            dir.display(),
        );
    }
    Ok(())
}

#[tracing::instrument(level = "debug", err)]
pub fn init_ockam_dir() -> anyhow::Result<std::path::PathBuf> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    let path = get_ockam_dir()?;
    tracing::debug!("Ockam dir will be at: {:?}", path);
    let parent = path.parent();
    anyhow::ensure!(
        !path.is_relative() || path.parent().is_none(),
        "Please use an absolute path if `$OCKAM_DIR` is manually specified.",
    );
    if !path.exists() {
        let parent = parent.unwrap();
        if !parent.exists() {
            eprintln!("Creating parent of ockam directory at: {:?}", path);
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create ockam directory's parent at `{parent:?}`")
            })?;
        }
        eprintln!("Creating ockam directory at {:?}", path);
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .with_context(|| {
                format!("failed to create path to ockam data and vault at `{path:?}/vault`")
            })?;
    } else {
        tracing::debug!("Ockam directory exists at {:?}.", path);
        let verify_mode = false;
        if verify_mode {
            let mode = path.metadata()?.mode();
            // Check that other users cannot modify the data inside.
            if (mode & 0o77) != 0 {
                eprintln!(
                    "Ockam directory at {:?} can be read/written by other users. Restricting...",
                    path
                );
                std::fs::set_permissions(&path, PermissionsExt::from_mode(0o700)).with_context(
                    || format!("failed to update permissions for ockam data at `{path:?}`"),
                )?;
            }
        }
    }
    Ok(path)
}

#[tracing::instrument(level = "debug", skip(data), err, fields(path = ?path))]
pub fn write(path: &std::path::Path, data: &[u8]) -> anyhow::Result<()> {
    use std::io::prelude::*;
    use std::os::unix::prelude::*;
    // TODO: look up how to avoid TOCTOU races for this case. Note that we must
    // still guarantee that there isn't a window where an unprivileged
    // process/user can read the data. Currently this has a race that results in
    // us failing to write, but theres no window where our mode could fail to be
    // used, which would be worse.
    if path.exists() {
        tracing::debug!("Note: removing previous file at {:?}", path);
        let _ = std::fs::remove_file(&path);
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .read(true)
        // `create_new` means we error if it exists. This ensures the mode we
        // provide is respect (the `mode(0o600)` is only used if creating the
        // file)
        .create_new(true)
        .mode(0o600) // TODO: not portable, what about windows?
        .open(&path)
        .with_context(|| format!("Failed to open file at {:?}", path))?;
    file.write_all(data)
        .with_context(|| format!("Failed to write file at {:?}", path))?;
    file.flush()
        .with_context(|| format!("could not flush {path:?}"))?;
    file.sync_all()
        .with_context(|| format!("could not fsync {path:?}"))?;
    Ok(())
}

pub fn load_trust_policy(
    ockam_dir: &std::path::Path,
) -> anyhow::Result<(TrustMultiIdentifiersPolicy, IdentityIdAccessControl)> {
    let path = ockam_dir.join("trusted");
    let idents = crate::old::identity::read_trusted_idents_from_file(&path)?;
    eprintln!(
        "Loaded {:?} trusted identifiers from list at '{}'",
        idents.len(),
        path.display(),
    );
    tracing::debug!("Trusting identifiers: {:?}", idents);

    let trust_policy = TrustMultiIdentifiersPolicy::new(idents.clone());
    let access_control = IdentityIdAccessControl::new(idents);

    Ok((trust_policy, access_control))
}
