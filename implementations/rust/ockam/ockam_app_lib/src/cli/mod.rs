use crate::{Error, Result};
use ockam_core::env::get_env;
use tracing::{error, info};

/// Return the ockam executable path either from the OCKAM env. variable or as `ockam`
pub(crate) fn cli_bin() -> Result<String> {
    let ockam_path = get_env("OCKAM")?;
    match ockam_path {
        Some(path) => Ok(path),
        None => {
            // check if the `ockam` command executable was bundled with the application
            let mut current_executable = std::env::current_exe()?;
            current_executable.pop();
            current_executable.push("ockam");
            match current_executable.into_os_string().into_string() {
                Ok(path) => {
                    if std::path::Path::new(&path).exists() {
                        Ok(path)
                    } else {
                        Ok("ockam".to_string())
                    }
                }
                Err(_) => Ok("ockam".to_string()),
            }
        }
    }
}

#[cfg(target_os = "macos")]
/// adds the homebrew path to the PATH environment variable
/// useful for development since the application is
/// bundled with the ockam_command executable
pub(crate) fn add_homebrew_to_path() {
    match std::env::var("PATH") {
        Ok(mut path) => {
            path.push(':');
            path.push_str("/opt/homebrew/bin/");
            std::env::set_var("PATH", path);
        }
        Err(_) => {
            tracing::debug!("PATH is not set");
        }
    }
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

    info!("Using ockam command path: {ockam_path}");

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
