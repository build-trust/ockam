use crate::util::parsers::duration_parser;
use clap::Args;
use ockam_core::env::get_env;
use ockam_multiaddr::MultiAddr;
use std::time::Duration;

#[derive(Clone, Debug, Args)]
pub struct IdentityOpts {
    /// Run the command as the given Identity
    #[arg(global = true, value_name = "IDENTITY_NAME", long = "identity")]
    pub identity_name: Option<String>,
}

#[derive(Clone, Debug, Args, Default, PartialEq)]
pub struct TrustOpts {
    /// Project name to use for the command
    #[arg(long = "project", value_name = "PROJECT_NAME")]
    pub project_name: Option<String>,

    /// Hex encoded Identity
    #[arg(long, value_name = "IDENTITY")]
    pub authority_identity: Option<String>,

    /// Address to the Authority node
    #[arg(long)]
    pub authority_route: Option<MultiAddr>,

    /// Expect credential manually saved to the storage
    #[arg(long)]
    pub credential_scope: Option<String>,
}

#[derive(Clone, Debug, Args, Default, PartialEq)]
pub struct RetryOpts {
    /// Number of times to retry the command
    #[arg(hide = true, long, alias = "retry")]
    retry_count: Option<u32>,

    /// Delay between retries
    #[arg(hide = true, long, value_parser = duration_parser)]
    retry_delay: Option<Duration>,

    /// Disable retry for the command,
    /// no matter if it's enabled via arguments or environment variables
    #[arg(hide = true, long, default_value_t = false)]
    no_retry: bool,
}

impl RetryOpts {
    /// Get the number of times to retry the command
    ///
    /// If the value is not set, it will try to get the value from
    /// the `OCKAM_COMMAND_RETRY_COUNT` environment variable
    pub fn retry_count(&self) -> Option<u32> {
        if self.no_retry {
            return None;
        }
        match self.retry_count {
            Some(count) => Some(count),
            None => get_env::<String>("OCKAM_COMMAND_RETRY_COUNT")
                .ok()
                .flatten()
                .and_then(|v| v.parse().ok()),
        }
    }

    /// Get the delay between retries
    ///
    /// If the value is not set, it will try to get the value from
    /// the `OCKAM_COMMAND_RETRY_DELAY` environment variable
    pub fn retry_delay(&self) -> Option<Duration> {
        if self.no_retry {
            return None;
        }
        match self.retry_delay {
            Some(delay) => Some(delay),
            None => get_env::<String>("OCKAM_COMMAND_RETRY_DELAY")
                .ok()
                .flatten()
                .and_then(|v| duration_parser(&v).ok()),
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct TimeoutArg {
    /// Override the default timeout duration that the command will wait for a response
    #[arg(long, value_name = "TIMEOUT", default_value = "5s", value_parser = duration_parser)]
    pub(crate) timeout: Duration,
}

#[derive(Debug, Clone, Args)]
pub struct OptionalTimeoutArg {
    /// Override the default timeout duration that the command will wait for a response
    #[arg(long, value_name = "TIMEOUT", default_value = "5s", value_parser = duration_parser)]
    pub(crate) timeout: Option<Duration>,
}
