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

use crate::admin::AdminCommand;
use crate::authority::{AuthorityCommand, AuthoritySubcommand};
use crate::command_global_opts::CommandGlobalOpts;
use crate::completion::CompletionCommand;
use crate::credential::CredentialCommand;
use crate::docs;
use crate::enroll::EnrollCommand;
use crate::environment::EnvironmentCommand;
use crate::flow_control::FlowControlCommand;
use crate::identity::IdentityCommand;
use crate::influxdb::inlet::InfluxDBInletCommand;
use crate::influxdb::outlet::InfluxDBOutletCommand;
use crate::kafka::consumer::KafkaConsumerCommand;
use crate::kafka::inlet::KafkaInletCommand;
use crate::kafka::outlet::KafkaOutletCommand;
use crate::kafka::producer::KafkaProducerCommand;
use crate::lease::LeaseCommand;
use crate::manpages::ManpagesCommand;
use crate::markdown::MarkdownCommand;
use crate::message::MessageCommand;
use crate::migrate_database::MigrateDatabaseCommand;
use crate::node::{NodeCommand, NodeSubcommand};
use crate::policy::PolicyCommand;
use crate::project::ProjectCommand;
use crate::project_admin::ProjectAdminCommand;
use crate::project_member::ProjectMemberCommand;
use crate::relay::RelayCommand;
use crate::rendezvous::RendezvousCommand;
use crate::reset::ResetCommand;
use crate::run::RunCommand;
use crate::secure_channel::listener::SecureChannelListenerCommand;
use crate::secure_channel::SecureChannelCommand;
use crate::service::ServiceCommand;
use crate::share::ShareCommand;
use crate::shared_args::RetryOpts;
use crate::sidecar::SidecarCommand;
use crate::space::SpaceCommand;
use crate::space_admin::SpaceAdminCommand;
use crate::status::StatusCommand;
use crate::subscription::SubscriptionCommand;
use crate::tcp::connection::TcpConnectionCommand;
use crate::tcp::inlet::TcpInletCommand;
use crate::tcp::listener::TcpListenerCommand;
use crate::tcp::outlet::TcpOutletCommand;
use crate::vault::VaultCommand;
use crate::worker::WorkerCommand;
use crate::Error;
use crate::Result;
use crate::{branding, branding::command};

#[derive(Clone, Debug, Subcommand)]
#[command(about = docs::about("List of commands which can be executed with `ockam`"))]
pub enum OckamSubcommand {
    #[command(name = command::name("enroll"), hide = command::hide("enroll"))]
    Enroll(EnrollCommand),

    #[command(name = command::name("node"), hide = command::hide("node"))]
    Node(NodeCommand),
    #[command(name = command::name("vault"), hide = command::hide("vault"))]
    Vault(VaultCommand),
    #[command(name = command::name("identity"), hide = command::hide("identity"))]
    Identity(IdentityCommand),
    #[command(name = command::name("project"), hide = command::hide("project"))]
    Project(ProjectCommand),
    #[command(name = command::name("policy"), hide = command::hide("policy"))]
    Policy(PolicyCommand),
    #[command(name = command::name("credential"), hide = command::hide("credential"))]
    Credential(CredentialCommand),
    #[command(name = command::name("relay"), hide = command::hide("relay"))]
    Relay(RelayCommand),
    #[command(name = command::name("tcp-outlet"), hide = command::hide("tcp-outlet"))]
    TcpOutlet(TcpOutletCommand),
    #[command(name = command::name("tcp-inlet"), hide = command::hide("tcp-inlet"))]
    TcpInlet(TcpInletCommand),
    #[command(name = command::name("kafka-inlet"), hide = command::hide("kafka-inlet"))]
    KafkaInlet(KafkaInletCommand),
    #[command(name = command::name("kafka-outlet"), hide = command::hide("kafka-outlet"))]
    KafkaOutlet(KafkaOutletCommand),
    #[command(name = command::name("influxdb-inlet"), hide = command::hide("influxdb-inlet"))]
    InfluxDBInlet(InfluxDBInletCommand),
    #[command(name = command::name("influxdb-outlet"), hide = command::hide("influxdb-outlet"))]
    InfluxDBOutlet(InfluxDBOutletCommand),
    #[command(name = command::name("rendezvous"), hide = command::hide("rendezvous") || docs::hide())]
    Rendezvous(RendezvousCommand),
    #[command(name = command::name("status"), hide = command::hide("status"))]
    Status(StatusCommand),
    #[command(name = command::name("reset"), hide = command::hide("reset"))]
    Reset(ResetCommand),
    #[command(name = command::name("run"), hide = command::hide("run"))]
    Run(RunCommand),
    #[command(name = command::name("manpages"), hide = command::hide("manpages"))]
    Manpages(ManpagesCommand),
    #[command(name = command::name("completion"), hide = command::hide("completion"))]
    Completion(CompletionCommand),
    #[command(name = command::name("environment"), hide = command::hide("environment"))]
    Environment(EnvironmentCommand),

    #[command(name = command::name("admin"), hide = command::hide("admin"))]
    Admin(AdminCommand),
    #[command(name = command::name("space"), hide = command::hide("space"))]
    Space(SpaceCommand),
    #[command(name = command::name("space-admin"), hide = command::hide("space-admin"))]
    SpaceAdmin(SpaceAdminCommand),
    #[command(name = command::name("project-admin"), hide = command::hide("project-admin"))]
    ProjectAdmin(ProjectAdminCommand),
    #[command(name = command::name("project-member"), hide = command::hide("project-member"))]
    ProjectMember(ProjectMemberCommand),
    #[command(name = command::name("sidecar"), hide = command::hide("sidecar"))]
    Sidecar(SidecarCommand),
    #[command(name = command::name("subscription"), hide = command::hide("subscription"))]
    Subscription(SubscriptionCommand),
    #[command(name = command::name("lease"), hide = command::hide("lease"))]
    Lease(LeaseCommand),
    #[command(name = command::name("authority"), hide = command::hide("authority"))]
    Authority(AuthorityCommand),
    #[command(name = command::name("service"), hide = command::hide("service"))]
    Service(ServiceCommand),
    #[command(name = command::name("message"), hide = command::hide("message"))]
    Message(MessageCommand),
    #[command(name = command::name("markdown"), hide = command::hide("markdown"))]
    Markdown(MarkdownCommand),

    #[command(name = command::name("migrate-database"), hide = command::hide("migrate-database"))]
    MigrateDatabase(MigrateDatabaseCommand),
    #[command(name = command::name("worker"), hide = command::hide("worker"))]
    Worker(WorkerCommand),
    #[command(name = command::name("secure-channel-listener"), hide = command::hide("secure-channel-listener"))]
    SecureChannelListener(SecureChannelListenerCommand),
    #[command(name = command::name("secure-channel"), hide = command::hide("secure-channel"))]
    SecureChannel(SecureChannelCommand),
    #[command(name = command::name("tcp-listener"), hide = command::hide("tcp-listener"))]
    TcpListener(TcpListenerCommand),
    #[command(name = command::name("tcp-connection"), hide = command::hide("tcp-connection"))]
    TcpConnection(TcpConnectionCommand),
    #[command(name = command::name("flow-control"), hide = command::hide("flow-control"))]
    FlowControl(FlowControlCommand),
    #[command(name = command::name("kafka-consumer"), hide = command::hide("kafka-consumer"))]
    KafkaConsumer(KafkaConsumerCommand),
    #[command(name = command::name("kafka-producer"), hide = command::hide("kafka-producer"))]
    KafkaProducer(KafkaProducerCommand),
    #[command(name = command::name("share"), hide = command::hide("share"))]
    Share(ShareCommand),
}

impl OckamSubcommand {
    /// Run the subcommand
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self {
            OckamSubcommand::Enroll(c) => c.run(ctx, opts).await,
            OckamSubcommand::Node(c) => c.run(ctx, opts).await,
            OckamSubcommand::Vault(c) => c.run(ctx, opts).await,
            OckamSubcommand::Identity(c) => c.run(ctx, opts).await,
            OckamSubcommand::Project(c) => c.run(ctx, opts).await,
            OckamSubcommand::Policy(c) => c.run(ctx, opts).await,
            OckamSubcommand::Credential(c) => c.run(ctx, opts).await,
            OckamSubcommand::Relay(c) => c.run(ctx, opts).await,
            OckamSubcommand::TcpOutlet(c) => c.run(ctx, opts).await,
            OckamSubcommand::TcpInlet(c) => c.run(ctx, opts).await,
            OckamSubcommand::KafkaInlet(c) => c.run(ctx, opts).await,
            OckamSubcommand::KafkaOutlet(c) => c.run(ctx, opts).await,
            OckamSubcommand::InfluxDBInlet(c) => c.run(ctx, opts).await,
            OckamSubcommand::InfluxDBOutlet(c) => c.run(ctx, opts).await,
            OckamSubcommand::Rendezvous(c) => c.run(ctx, opts).await,
            OckamSubcommand::Status(c) => c.run(ctx, opts).await,
            OckamSubcommand::Reset(c) => c.run(ctx, opts).await,
            OckamSubcommand::Run(c) => c.run(ctx, opts).await,

            OckamSubcommand::Manpages(c) => c.run(),
            OckamSubcommand::Completion(c) => c.run(),
            OckamSubcommand::Environment(c) => c.run(),

            OckamSubcommand::Admin(c) => c.run(ctx, opts).await,
            OckamSubcommand::Space(c) => c.run(ctx, opts).await,
            OckamSubcommand::SpaceAdmin(c) => c.run(ctx, opts).await,
            OckamSubcommand::ProjectAdmin(c) => c.run(ctx, opts).await,
            OckamSubcommand::ProjectMember(c) => c.run(ctx, opts).await,
            OckamSubcommand::Sidecar(c) => c.run(ctx, opts).await,
            OckamSubcommand::Subscription(c) => c.run(ctx, opts).await,
            OckamSubcommand::Lease(c) => c.run(ctx, opts).await,
            OckamSubcommand::Authority(c) => c.run(ctx, opts).await,
            OckamSubcommand::Service(c) => c.run(ctx, opts).await,
            OckamSubcommand::Message(c) => c.run(ctx, opts).await,
            OckamSubcommand::Markdown(c) => c.run(),

            OckamSubcommand::MigrateDatabase(c) => c.run(ctx, opts).await,
            OckamSubcommand::Worker(c) => c.run(ctx, opts).await,
            OckamSubcommand::SecureChannelListener(c) => c.run(ctx, opts).await,
            OckamSubcommand::SecureChannel(c) => c.run(ctx, opts).await,
            OckamSubcommand::TcpListener(c) => c.run(ctx, opts).await,
            OckamSubcommand::TcpConnection(c) => c.run(ctx, opts).await,
            OckamSubcommand::FlowControl(c) => c.run(ctx, opts).await,
            OckamSubcommand::KafkaConsumer(c) => c.run(ctx, opts).await,
            OckamSubcommand::KafkaProducer(c) => c.run(ctx, opts).await,
            OckamSubcommand::Share(c) => c.run(ctx, opts).await,
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
            OckamSubcommand::Enroll(c) => c.name(),
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
            OckamSubcommand::Admin(c) => c.name(),
            OckamSubcommand::Space(c) => c.name(),
            OckamSubcommand::SpaceAdmin(c) => c.name(),
            OckamSubcommand::ProjectAdmin(c) => c.name(),
            OckamSubcommand::ProjectMember(c) => c.name(),
            OckamSubcommand::Sidecar(c) => c.name(),
            OckamSubcommand::Subscription(c) => c.name(),
            OckamSubcommand::Lease(c) => c.name(),
            OckamSubcommand::Authority(c) => c.name(),
            OckamSubcommand::Markdown(c) => c.name(),
            OckamSubcommand::MigrateDatabase(c) => c.name(),
            OckamSubcommand::Worker(c) => c.name(),
            OckamSubcommand::Service(c) => c.name(),
            OckamSubcommand::Message(c) => c.name(),
            OckamSubcommand::SecureChannelListener(c) => c.name(),
            OckamSubcommand::SecureChannel(c) => c.name(),
            OckamSubcommand::TcpListener(c) => c.name(),
            OckamSubcommand::TcpConnection(c) => c.name(),
            OckamSubcommand::FlowControl(c) => c.name(),
            OckamSubcommand::KafkaConsumer(c) => c.name(),
            OckamSubcommand::KafkaProducer(c) => c.name(),
            OckamSubcommand::Share(c) => c.name(),
        }
    }
}

#[async_trait]
pub trait Command: Debug + Clone + Sized + Send + Sync + 'static {
    const NAME: &'static str;

    fn name(&self) -> String {
        branding::command::name(Self::NAME).to_string()
    }

    fn hide() -> bool {
        branding::command::hide(Self::NAME)
    }

    fn retry_opts(&self) -> Option<RetryOpts> {
        None
    }

    async fn run_with_retry(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        if let Some(retry_opts) = self.retry_opts() {
            let (mut retry_count, retry_delay) =
                match (retry_opts.retry_count(), retry_opts.retry_delay()) {
                    (Some(count), Some(delay)) => (count, delay),
                    (Some(count), None) => (count, Duration::from_secs(5)),
                    (None, Some(delay)) => (3, delay),
                    (None, None) => {
                        self.run(ctx, opts).await?;
                        return Ok(());
                    }
                };
            let retry_delay_jitter = min(
                Duration::from_secs_f64(retry_delay.as_secs_f64() * 0.5),
                Duration::from_secs(5),
            );
            while retry_count > 0 {
                let cmd = self.clone();
                match cmd.run(ctx, opts.clone()).await {
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
            self.run(ctx, opts).await?;
            Ok(())
        }
    }

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()>;
}
