use std::cmp::min;
use std::fmt::Debug;
use std::ops::Add;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use clap::Subcommand;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio_retry::strategy::jitter;
use tracing::warn;

use ockam_api::{fmt_log, fmt_warn, CliState};
use ockam_core::OpenTelemetryContext;
use ockam_node::Context;

use crate::command_global_opts::CommandGlobalOpts;
use crate::completion::CompletionCommand;
use crate::credential::CredentialCommand;
use crate::docs;
use crate::environment::EnvironmentCommand;
use crate::identity::IdentityCommand;
use crate::influxdb::inlet::InfluxDBInletCommand;
use crate::influxdb::outlet::InfluxDBOutletCommand;
use crate::kafka::inlet::KafkaInletCommand;
use crate::kafka::outlet::KafkaOutletCommand;
use crate::manpages::ManpagesCommand;
use crate::node::{NodeCommand, NodeSubcommand};
use crate::policy::PolicyCommand;
use crate::project::ProjectCommand;
use crate::relay::RelayCommand;
use crate::rendezvous::RendezvousCommand;
use crate::reset::ResetCommand;
use crate::run::RunCommand;
use crate::shared_args::RetryOpts;
use crate::status::StatusCommand;
use crate::tcp::inlet::TcpInletCommand;
use crate::tcp::outlet::TcpOutletCommand;
use crate::util::async_cmd;
use crate::vault::VaultCommand;
use crate::Error;
use crate::Result;

cfg_if::cfg_if! {
    if #[cfg(feature = "admin_commands")] {
        use crate::enroll::EnrollCommand;
        use crate::admin::AdminCommand;
        use crate::authority::{AuthorityCommand, AuthoritySubcommand};
        use crate::lease::LeaseCommand;
        use crate::markdown::MarkdownCommand;
        use crate::project_admin::ProjectAdminCommand;
        use crate::project_member::ProjectMemberCommand;
        use crate::sidecar::SidecarCommand;
        use crate::space::SpaceCommand;
        use crate::space_admin::SpaceAdminCommand;
        use crate::subscription::SubscriptionCommand;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "advanced_commands")] {
        use crate::flow_control::FlowControlCommand;
        use crate::kafka::consumer::KafkaConsumerCommand;
        use crate::kafka::producer::KafkaProducerCommand;
        use crate::message::MessageCommand;
        use crate::secure_channel::listener::SecureChannelListenerCommand;
        use crate::secure_channel::SecureChannelCommand;
        use crate::service::ServiceCommand;
        use crate::share::ShareCommand;
        use crate::tcp::listener::TcpListenerCommand;
        use crate::tcp::connection::TcpConnectionCommand;
        use crate::worker::WorkerCommand;
    }
}

#[derive(Clone, Debug, Subcommand)]
#[command(about = docs::about("List of commands which can be executed with `ockam`"))]
pub enum OckamSubcommand {
    Node(NodeCommand),
    Vault(VaultCommand),
    Identity(IdentityCommand),
    Project(ProjectCommand),
    Policy(PolicyCommand),
    Credential(CredentialCommand),
    Relay(RelayCommand),
    TcpOutlet(TcpOutletCommand),
    TcpInlet(TcpInletCommand),
    KafkaInlet(KafkaInletCommand),
    KafkaOutlet(KafkaOutletCommand),
    #[command(name = "influxdb-inlet")]
    InfluxDBInlet(InfluxDBInletCommand),
    #[command(name = "influxdb-outlet")]
    InfluxDBOutlet(InfluxDBOutletCommand),
    #[command(hide = docs::hide())]
    Rendezvous(RendezvousCommand),
    Status(StatusCommand),
    Reset(ResetCommand),
    Run(RunCommand),
    Manpages(ManpagesCommand),
    Completion(CompletionCommand),
    Environment(EnvironmentCommand),

    #[cfg(feature = "admin_commands")]
    Enroll(EnrollCommand),
    #[cfg(feature = "admin_commands")]
    Admin(AdminCommand),
    #[cfg(feature = "admin_commands")]
    Space(SpaceCommand),
    #[cfg(feature = "admin_commands")]
    SpaceAdmin(SpaceAdminCommand),
    #[cfg(feature = "admin_commands")]
    ProjectAdmin(ProjectAdminCommand),
    #[cfg(feature = "admin_commands")]
    ProjectMember(ProjectMemberCommand),

    #[cfg(feature = "admin_commands")]
    Sidecar(SidecarCommand),
    #[cfg(feature = "admin_commands")]
    Subscription(SubscriptionCommand),
    #[cfg(feature = "admin_commands")]
    Lease(LeaseCommand),
    #[cfg(feature = "admin_commands")]
    Authority(AuthorityCommand),

    #[cfg(feature = "admin_commands")]
    Markdown(MarkdownCommand),

    #[cfg(feature = "advanced_commands")]
    Worker(WorkerCommand),
    #[cfg(feature = "advanced_commands")]
    Service(ServiceCommand),
    #[cfg(feature = "advanced_commands")]
    Message(MessageCommand),

    #[cfg(feature = "advanced_commands")]
    SecureChannelListener(SecureChannelListenerCommand),
    #[cfg(feature = "advanced_commands")]
    SecureChannel(SecureChannelCommand),
    #[cfg(feature = "advanced_commands")]
    TcpListener(TcpListenerCommand),
    #[cfg(feature = "advanced_commands")]
    TcpConnection(TcpConnectionCommand),
    #[cfg(feature = "advanced_commands")]
    FlowControl(FlowControlCommand),

    #[cfg(feature = "advanced_commands")]
    KafkaConsumer(KafkaConsumerCommand),
    #[cfg(feature = "advanced_commands")]
    KafkaProducer(KafkaProducerCommand),
    #[cfg(feature = "advanced_commands")]
    Share(ShareCommand),
}

impl OckamSubcommand {
    /// Run the subcommand
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self {
            OckamSubcommand::Node(c) => c.run(opts),
            OckamSubcommand::Vault(c) => c.run(opts),
            OckamSubcommand::Identity(c) => c.run(opts),
            OckamSubcommand::Project(c) => c.run(opts),
            OckamSubcommand::Policy(c) => c.run(opts),
            OckamSubcommand::Credential(c) => c.run(opts),
            OckamSubcommand::Relay(c) => c.run(opts),
            OckamSubcommand::TcpOutlet(c) => c.run(opts),
            OckamSubcommand::TcpInlet(c) => c.run(opts),
            OckamSubcommand::KafkaInlet(c) => c.run(opts),
            OckamSubcommand::KafkaOutlet(c) => c.run(opts),
            OckamSubcommand::InfluxDBInlet(c) => c.run(opts),
            OckamSubcommand::InfluxDBOutlet(c) => c.run(opts),
            OckamSubcommand::Rendezvous(c) => c.run(opts),
            OckamSubcommand::Status(c) => c.run(opts),
            OckamSubcommand::Reset(c) => c.run(opts),
            OckamSubcommand::Run(c) => c.run(opts),
            OckamSubcommand::Manpages(c) => c.run(),
            OckamSubcommand::Completion(c) => c.run(),
            OckamSubcommand::Environment(c) => c.run(),

            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Enroll(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Admin(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Space(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::SpaceAdmin(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::ProjectAdmin(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::ProjectMember(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Sidecar(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Subscription(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Lease(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Authority(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Markdown(c) => c.run(),

            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::Worker(c) => c.run(opts),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::Service(c) => c.run(opts),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::Message(c) => c.run(opts),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::SecureChannelListener(c) => c.run(opts),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::SecureChannel(c) => c.run(opts),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::TcpListener(c) => c.run(opts),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::TcpConnection(c) => c.run(opts),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::FlowControl(c) => c.run(opts),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::KafkaConsumer(c) => c.run(opts),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::KafkaProducer(c) => c.run(opts),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::Share(c) => c.run(opts),
        }
    }

    /// Return the opentelemetry context if the command can be executed as the continuation
    /// of an existing trace
    pub fn get_opentelemetry_context(&self) -> Option<OpenTelemetryContext> {
        match self {
            OckamSubcommand::Node(cmd) => match &cmd.subcommand {
                NodeSubcommand::Create(cmd) => cmd.opentelemetry_context.clone(),
                _ => None,
            },
            _ => None,
        }
    }

    /// Return true if this command represents the execution of a foreground node
    pub fn is_foreground_node(&self) -> bool {
        match self {
            OckamSubcommand::Node(cmd) => match &cmd.subcommand {
                NodeSubcommand::Create(cmd) => !cmd.foreground_args.child_process,
                _ => false,
            },
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Authority(cmd) => match &cmd.subcommand {
                AuthoritySubcommand::Create(cmd) => !cmd.child_process,
            },
            _ => false,
        }
    }

    /// Return true if this command represents the execution of a background node
    pub fn is_background_node(&self) -> bool {
        match self {
            OckamSubcommand::Node(cmd) => match &cmd.subcommand {
                NodeSubcommand::Create(cmd) => cmd.foreground_args.child_process,
                _ => false,
            },
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Authority(cmd) => match &cmd.subcommand {
                AuthoritySubcommand::Create(cmd) => cmd.child_process,
            },
            _ => false,
        }
    }

    /// Return the node name for an ockam node create command
    pub fn node_name(&self) -> Option<String> {
        match self {
            OckamSubcommand::Node(cmd) => match &cmd.subcommand {
                NodeSubcommand::Create(cmd) => {
                    if cmd.foreground_args.child_process {
                        Some(cmd.name.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            },
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Authority(cmd) => match &cmd.subcommand {
                AuthoritySubcommand::Create(cmd) => {
                    if cmd.child_process {
                        Some(cmd.node_name())
                    } else {
                        None
                    }
                }
            },
            _ => None,
        }
    }

    /// Return a path if the command requires the creation of log files in a specific directory
    pub fn log_path(&self) -> Option<PathBuf> {
        match self {
            OckamSubcommand::Node(cmd) => match &cmd.subcommand {
                NodeSubcommand::Create(cmd) => {
                    if cmd.foreground_args.child_process || !cmd.foreground_args.foreground {
                        CliState::default_node_dir(&cmd.name).ok()
                    } else {
                        None
                    }
                }
                _ => None,
            },
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Authority(cmd) => match &cmd.subcommand {
                AuthoritySubcommand::Create(cmd) => {
                    if cmd.child_process || !cmd.foreground {
                        CliState::default_node_dir(&cmd.node_name()).ok()
                    } else {
                        None
                    }
                }
            },
            _ => None,
        }
    }

    /// Return the subcommand name
    pub fn name(&self) -> String {
        match self {
            OckamSubcommand::Node(c) => c.name(),
            OckamSubcommand::Vault(c) => c.name(),
            OckamSubcommand::Identity(c) => c.name(),
            OckamSubcommand::Project(c) => c.name(),
            OckamSubcommand::Policy(c) => c.name(),
            OckamSubcommand::Credential(c) => c.name(),
            OckamSubcommand::Relay(c) => c.name(),
            OckamSubcommand::TcpOutlet(c) => c.name(),
            OckamSubcommand::TcpInlet(c) => c.name(),
            OckamSubcommand::KafkaInlet(c) => c.name(),
            OckamSubcommand::KafkaOutlet(c) => c.name(),
            OckamSubcommand::InfluxDBInlet(c) => c.name(),
            OckamSubcommand::InfluxDBOutlet(c) => c.name(),
            OckamSubcommand::Rendezvous(c) => c.name(),
            OckamSubcommand::Status(c) => c.name(),
            OckamSubcommand::Reset(c) => c.name(),
            OckamSubcommand::Run(c) => c.name(),
            OckamSubcommand::Manpages(c) => c.name(),
            OckamSubcommand::Completion(c) => c.name(),
            OckamSubcommand::Environment(c) => c.name(),

            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Enroll(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Admin(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Space(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::SpaceAdmin(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::ProjectAdmin(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::ProjectMember(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Sidecar(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Subscription(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Lease(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Authority(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            OckamSubcommand::Markdown(c) => c.name(),

            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::Worker(c) => c.name(),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::Service(c) => c.name(),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::Message(c) => c.name(),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::SecureChannelListener(c) => c.name(),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::SecureChannel(c) => c.name(),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::TcpListener(c) => c.name(),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::TcpConnection(c) => c.name(),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::FlowControl(c) => c.name(),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::KafkaConsumer(c) => c.name(),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::KafkaProducer(c) => c.name(),
            #[cfg(feature = "advanced_commands")]
            OckamSubcommand::Share(c) => c.name(),
        }
    }
}

#[async_trait]
pub trait Command: Debug + Clone + Sized + Send + Sync + 'static {
    const NAME: &'static str;

    fn name(&self) -> String {
        Self::NAME.into()
    }

    fn retry_opts(&self) -> Option<RetryOpts> {
        None
    }

    fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(Self::NAME, opts.clone(), |ctx| async move {
            self.async_run_with_retry(&ctx, opts).await
        })
    }

    async fn async_run_with_retry(
        self,
        ctx: &Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        if let Some(retry_opts) = self.retry_opts() {
            let (mut retry_count, retry_delay) =
                match (retry_opts.retry_count(), retry_opts.retry_delay()) {
                    (Some(count), Some(delay)) => (count, delay),
                    (Some(count), None) => (count, Duration::from_secs(5)),
                    (None, Some(delay)) => (3, delay),
                    (None, None) => {
                        self.async_run(ctx, opts).await?;
                        return Ok(());
                    }
                };
            let retry_delay_jitter = min(
                Duration::from_secs_f64(retry_delay.as_secs_f64() * 0.5),
                Duration::from_secs(5),
            );
            while retry_count > 0 {
                let cmd = self.clone();
                match cmd.async_run(ctx, opts.clone()).await {
                    Ok(_) => break,
                    Err(report) => {
                        match report.downcast::<Error>() {
                            Ok(error) => {
                                match error {
                                    Error::Retry(report) => {
                                        retry_count -= 1;
                                        // return the last error if there are no more retries
                                        if retry_count == 0 {
                                            return Err(report);
                                        };

                                        let delay = retry_delay.add(jitter(retry_delay_jitter));
                                        warn!(
                                            "Command failed, retrying in {} seconds: {report:?}",
                                            delay.as_secs()
                                        );
                                        opts.terminal
                                            .write_line(fmt_warn!("Command failed with error:"))?;
                                        opts.terminal.write_line(fmt_log!("{report:#}\n"))?;
                                        opts.terminal.write_line(fmt_log!(
                                            "Will retry in {} seconds",
                                            delay.as_secs()
                                        ))?;
                                        tokio::time::sleep(delay).await;
                                        opts.terminal.write_line(fmt_log!("Retrying...\n"))?;
                                    }
                                    error => return Err(error).into_diagnostic(),
                                }
                            }
                            Err(report) => {
                                return Err(report);
                            }
                        }
                    }
                }
            }
            Ok(())
        } else {
            self.async_run(ctx, opts).await?;
            Ok(())
        }
    }

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()>;
}
