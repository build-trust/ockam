use clap::Subcommand;
use std::path::PathBuf;

use ockam_api::CliState;
use ockam_core::OpenTelemetryContext;

use crate::admin::AdminCommand;
use crate::authority::{AuthorityCommand, AuthoritySubcommand};
use crate::command_global_opts::CommandGlobalOpts;
use crate::completion::CompletionCommand;
use crate::configuration::ConfigurationCommand;
use crate::credential::CredentialCommand;
use crate::enroll::EnrollCommand;
use crate::environment::EnvironmentCommand;
use crate::flow_control::FlowControlCommand;
use crate::identity::IdentityCommand;
use crate::kafka::consumer::KafkaConsumerCommand;
use crate::kafka::direct::KafkaDirectCommand;
use crate::kafka::outlet::KafkaOutletCommand;
use crate::kafka::producer::KafkaProducerCommand;
use crate::lease::LeaseCommand;
use crate::manpages::ManpagesCommand;
use crate::markdown::MarkdownCommand;
use crate::message::MessageCommand;
use crate::node::NodeCommand;
use crate::node::NodeSubcommand;
use crate::policy::PolicyCommand;
use crate::project::ProjectCommand;
use crate::relay::RelayCommand;
use crate::reset::ResetCommand;
use crate::run::RunCommand;
use crate::secure_channel::listener::SecureChannelListenerCommand;
use crate::secure_channel::SecureChannelCommand;
use crate::service::ServiceCommand;
#[cfg(feature = "orchestrator")]
use crate::share::ShareCommand;
use crate::sidecar::SidecarCommand;
use crate::space::SpaceCommand;
use crate::status::StatusCommand;
use crate::subscription::SubscriptionCommand;
use crate::tcp::connection::TcpConnectionCommand;
use crate::tcp::inlet::TcpInletCommand;
use crate::tcp::listener::TcpListenerCommand;
use crate::tcp::outlet::TcpOutletCommand;
use crate::vault::VaultCommand;
use crate::worker::WorkerCommand;

/// List of commands which can be executed with `ockam`
#[derive(Clone, Debug, Subcommand)]
pub enum OckamSubcommand {
    #[command(display_order = 800)]
    Enroll(EnrollCommand),
    Space(SpaceCommand),
    Project(ProjectCommand),
    Sidecar(SidecarCommand),
    Admin(AdminCommand),
    #[cfg(feature = "orchestrator")]
    Share(ShareCommand),
    Subscription(SubscriptionCommand),

    Node(Box<NodeCommand>),
    Worker(WorkerCommand),
    Service(ServiceCommand),
    Message(MessageCommand),
    Relay(RelayCommand),

    TcpListener(TcpListenerCommand),
    TcpConnection(TcpConnectionCommand),
    TcpOutlet(TcpOutletCommand),
    TcpInlet(TcpInletCommand),

    KafkaOutlet(KafkaOutletCommand),
    KafkaConsumer(KafkaConsumerCommand),
    KafkaDirect(KafkaDirectCommand),
    KafkaProducer(KafkaProducerCommand),

    SecureChannelListener(SecureChannelListenerCommand),
    SecureChannel(SecureChannelCommand),

    Vault(VaultCommand),
    Identity(IdentityCommand),
    Credential(CredentialCommand),
    Authority(AuthorityCommand),
    Policy(PolicyCommand),
    Lease(LeaseCommand),

    Run(RunCommand),
    Status(StatusCommand),
    Reset(ResetCommand),
    Configuration(ConfigurationCommand),

    Completion(CompletionCommand),
    Markdown(MarkdownCommand),
    Manpages(ManpagesCommand),
    Environment(EnvironmentCommand),

    FlowControl(FlowControlCommand),
}

impl OckamSubcommand {
    /// Run the subcommand
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self {
            OckamSubcommand::Enroll(c) => c.run(opts),
            OckamSubcommand::Space(c) => c.run(opts),
            OckamSubcommand::Project(c) => c.run(opts),
            OckamSubcommand::Admin(c) => c.run(opts),
            #[cfg(feature = "orchestrator")]
            OckamSubcommand::Share(c) => c.run(opts),
            OckamSubcommand::Subscription(c) => c.run(opts),

            OckamSubcommand::Node(c) => c.run(opts),
            OckamSubcommand::Worker(c) => c.run(opts),
            OckamSubcommand::Service(c) => c.run(opts),
            OckamSubcommand::Message(c) => c.run(opts),
            OckamSubcommand::Relay(c) => c.run(opts),

            OckamSubcommand::KafkaOutlet(c) => c.run(opts),
            OckamSubcommand::TcpListener(c) => c.run(opts),
            OckamSubcommand::TcpConnection(c) => c.run(opts),
            OckamSubcommand::TcpOutlet(c) => c.run(opts),
            OckamSubcommand::TcpInlet(c) => c.run(opts),

            OckamSubcommand::KafkaConsumer(c) => c.run(opts),
            OckamSubcommand::KafkaProducer(c) => c.run(opts),
            OckamSubcommand::KafkaDirect(c) => c.run(opts),

            OckamSubcommand::SecureChannelListener(c) => c.run(opts),
            OckamSubcommand::SecureChannel(c) => c.run(opts),

            OckamSubcommand::Vault(c) => c.run(opts),
            OckamSubcommand::Identity(c) => c.run(opts),
            OckamSubcommand::Credential(c) => c.run(opts),
            OckamSubcommand::Authority(c) => c.run(opts),
            OckamSubcommand::Policy(c) => c.run(opts),
            OckamSubcommand::Lease(c) => c.run(opts),

            OckamSubcommand::Run(c) => c.run(opts),
            OckamSubcommand::Status(c) => c.run(opts),
            OckamSubcommand::Reset(c) => c.run(opts),
            OckamSubcommand::Configuration(c) => c.run(opts),

            OckamSubcommand::Completion(c) => c.run(),
            OckamSubcommand::Markdown(c) => c.run(),
            OckamSubcommand::Manpages(c) => c.run(),
            OckamSubcommand::Environment(c) => c.run(),

            OckamSubcommand::FlowControl(c) => c.run(opts),
            OckamSubcommand::Sidecar(c) => c.run(opts),
        }
    }

    /// Currently only enroll command displays the header
    pub fn should_display_header(&self) -> bool {
        matches!(self, OckamSubcommand::Enroll(_))
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

    /// Return true if this command represents the execution of a background node
    pub fn is_background_node(&self) -> bool {
        match self {
            OckamSubcommand::Node(cmd) => match &cmd.subcommand {
                NodeSubcommand::Create(cmd) => cmd.child_process,
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
                    if cmd.child_process {
                        Some(cmd.node_name.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            },
            OckamSubcommand::Authority(cmd) => match &cmd.subcommand {
                AuthoritySubcommand::Create(cmd) => {
                    if cmd.child_process {
                        Some(cmd.node_name.clone())
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
                    if cmd.child_process {
                        CliState::default_node_dir(&cmd.node_name).ok()
                    } else {
                        None
                    }
                }
                _ => None,
            },
            OckamSubcommand::Authority(cmd) => match &cmd.subcommand {
                AuthoritySubcommand::Create(cmd) => {
                    if cmd.child_process {
                        CliState::default_node_dir(&cmd.node_name).ok()
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
            OckamSubcommand::Enroll(c) => c.name(),
            OckamSubcommand::Space(c) => c.name(),
            OckamSubcommand::Project(c) => c.name(),
            OckamSubcommand::Sidecar(c) => c.name(),
            OckamSubcommand::Admin(c) => c.name(),
            OckamSubcommand::Share(c) => c.name(),
            OckamSubcommand::Subscription(c) => c.name(),
            OckamSubcommand::Worker(c) => c.name(),
            OckamSubcommand::Service(c) => c.name(),
            OckamSubcommand::Message(c) => c.name(),
            OckamSubcommand::Relay(c) => c.name(),
            OckamSubcommand::TcpListener(c) => c.name(),
            OckamSubcommand::TcpConnection(c) => c.name(),
            OckamSubcommand::TcpOutlet(c) => c.name(),
            OckamSubcommand::TcpInlet(c) => c.name(),
            OckamSubcommand::KafkaOutlet(c) => c.name(),
            OckamSubcommand::KafkaConsumer(c) => c.name(),
            OckamSubcommand::KafkaDirect(c) => c.name(),
            OckamSubcommand::KafkaProducer(c) => c.name(),
            OckamSubcommand::SecureChannelListener(c) => c.name(),
            OckamSubcommand::SecureChannel(c) => c.name(),
            OckamSubcommand::Vault(c) => c.name(),
            OckamSubcommand::Identity(c) => c.name(),
            OckamSubcommand::Credential(c) => c.name(),
            OckamSubcommand::Authority(c) => c.name(),
            OckamSubcommand::Policy(c) => c.name(),
            OckamSubcommand::Lease(c) => c.name(),
            OckamSubcommand::Run(c) => c.name(),
            OckamSubcommand::Status(c) => c.name(),
            OckamSubcommand::Reset(c) => c.name(),
            OckamSubcommand::Configuration(c) => c.name(),
            OckamSubcommand::Completion(c) => c.name(),
            OckamSubcommand::Markdown(c) => c.name(),
            OckamSubcommand::Manpages(c) => c.name(),
            OckamSubcommand::Environment(c) => c.name(),
            OckamSubcommand::FlowControl(c) => c.name(),
        }
    }
}
