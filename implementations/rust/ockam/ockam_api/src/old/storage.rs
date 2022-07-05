use crate::error::ApiError;
use ockam_core::Result;

#[tracing::instrument(level = "debug", skip(data), err, fields(path = ?path))]
pub fn write(path: &std::path::Path, data: &[u8]) -> Result<()> {
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
        .map_err(|_| ApiError::generic(&format!("Failed to open file at {:?}", path)))?;
    file.write_all(data)
        .map_err(|_| ApiError::generic(&format!("Failed to write file at {:?}", path)))?;
    file.flush()
        .map_err(|_| ApiError::generic(&format!("could not flush {path:?}")))?;
    file.sync_all()
        .map_err(|_| ApiError::generic(&format!("could not fsync {path:?}")))?;
    Ok(())
}
