use crate::{Error, Result};
use ockam_core::env::get_env_with_default;
use tracing::{error, info};

/// Return the ockam executable path either from the OCKAM env. variable or as `ockam`
pub(crate) fn cli_bin() -> Result<String> {
    let ockam_path = get_env_with_default("OCKAM", "ockam".to_string())?;
    Ok(ockam_path)
}

/// Check that the OCKAM environment variable defines an absolute path
/// Otherwise we might fail to run the ockam command when starting the desktop application from an unexpected path
/// Check that the ockam command can at least be called with the `--version` option and log
/// its version
pub(crate) fn check_ockam_executable() -> Result<()> {
    // Get the ockam executable path and check that it is an absolute path
    let ockam_path = cli_bin()?;
    if ockam_path != *"ockam" && !ockam_path.starts_with('/') {
        let message = format!("The OCKAM environment variable must be defined with an absolute path. The current value is: {ockam_path}");
        error!(message);
        return Err(Error::App(message));
    };

    #[cfg(target_os = "macos")]
    {
        // HACK: ockam is not installed in a listed PATH on MacOS when installed with homebrew
        // On MacOS, the ockam command is installed in /opt/homebrew/bin/ockam
        // It won't work if the user installed homebrew in a different location
        let mut paths = std::env::var_os("PATH").unwrap_or_default();
        paths.push(":");
        paths.push("/opt/homebrew/bin/");
        std::env::set_var("PATH", &paths);
    }

    // Check that the executable can be found on the path
    match duct::cmd!("which", ockam_path.clone())
        .stderr_null()
        .stdout_capture()
        .run()
    {
        Err(e) => {
            let message = format!("The ockam path could not be found: {e}");
            error!(message);
            return Err(Error::App(message));
        }
        Ok(v) => info!(
            "The ockam command was found at {:?}",
            std::str::from_utf8(&v.stdout)
                .unwrap_or("can't decode the ockam path")
                .split('\n')
                .collect::<Vec<&str>>()
                .join(" ")
        ),
    };

    // Get the command line version
    match duct::cmd!(ockam_path, "--version")
        .stderr_null()
        .stdout_capture()
        .run()
    {
        Err(e) => {
            let message = format!("The ockam command could not be executed correctly: {e}. Please execute $OCKAM --version or ockam --version");
            error!(message);
            return Err(Error::App(message));
        }
        Ok(v) => info!(
            "The ockam command is available {:?}",
            std::str::from_utf8(&v.stdout)
                .unwrap_or("can't decode the ockam version")
                .split('\n')
                .collect::<Vec<&str>>()
                .join(" ")
        ),
    }
    Ok(())
}
