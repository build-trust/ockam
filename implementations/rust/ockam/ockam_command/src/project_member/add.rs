use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::direct::Members;
use ockam_api::nodes::InMemoryNode;
use ockam_multiaddr::MultiAddr;

use crate::project_member::{create_authority_client, create_member_attributes, get_project};
use crate::util::api::{IdentityOpts, RetryOpts};
use crate::{docs, fmt_ok, Command, CommandGlobalOpts, Error};

const LONG_ABOUT: &str = include_str!("./static/add/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/add/after_long_help.txt");

/// This attribute in credential allows member to create a relay on the Project node, the name of the relay should be
/// equal to the value of that attribute. If the value is `*` then any name is allowed
pub const OCKAM_RELAY_ATTRIBUTE: &str = "ockam-relay";

/// Add members to a Project, as an authorized enroller directly
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct AddCommand {
    #[command(flatten)]
    identity_opts: IdentityOpts,

    /// Which project add member to
    #[arg(long, short, value_name = "ROUTE_TO_PROJECT")]
    to: Option<MultiAddr>,

    #[arg(value_name = "IDENTIFIER")]
    member: Identifier,

    /// Attributes in `key=value` format to be attached to the member. You can specify this option multiple times for multiple attributes
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    attributes: Vec<String>,

    /// Name of the relay that the identity will be allowed to create. This name is transformed into attributes to prevent collisions when creating relay names. For example: `--relay foo` is shorthand for `--attribute ockam-relay=foo`
    #[arg(long = "relay", value_name = "ENROLLEE_ALLOWED_RELAY_NAME")]
    allowed_relay_name: Option<String>,

    /// Add the enroller role. If you specify it, this flag is transformed into the attributes `--attribute ockam-role=enroller`. This role allows the Identity using the ticket to enroll other Identities into the Project, typically something that only admins can do
    #[arg(long = "enroller")]
    enroller: bool,

    #[command(flatten)]
    retry_opts: RetryOpts,
}

#[async_trait]
impl Command for AddCommand {
    const NAME: &'static str = "project-member add";

    fn retry_opts(&self) -> Option<RetryOpts> {
        Some(self.retry_opts.clone())
    }

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let project = get_project(&opts.state, &self.to).await?;

        let node = InMemoryNode::start_with_project_name(
            ctx,
            &opts.state,
            Some(project.name().to_string()),
        )
        .await?;

        let authority_node_client =
            create_authority_client(&node, &opts.state, &self.identity_opts, &project).await?;

        authority_node_client
            .add_member(
                ctx,
                self.member.clone(),
                create_member_attributes(
                    &self.attributes,
                    &self.allowed_relay_name,
                    self.enroller,
                )?,
            )
            .await
            .map_err(Error::Retry)?;

        opts.terminal.stdout().plain(fmt_ok!(
            "Identifier {} is now a Project member. It can get a credential and access Project resources, like portals of other members",
            self.member
        ));

        Ok(())
    }
}
