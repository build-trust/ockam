use abac_examples::fixtures;

use ockam::abac::{
    self, AbacAttributeStorage, AbacAuthorization, AbacPolicyStorage, Action, Method, Resource,
    Subject,
};
use ockam::{
    authenticated_storage::InMemoryStorage,
    identity::{Identity, IdentitySecureChannelLocalInfo, TrustEveryonePolicy},
    vault::Vault,
};
use ockam::{route, Address, Context, Result, Routed, Worker, WorkerBuilder};
use ockam_core::{
    async_trait,
    compat::{boxed::Box, sync::Arc},
};

// - e.g. Some ockam_api implementation ---------------------------------------

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Get a-hold of some secure channel
    let some_secure_channel = mock_ockam_cloud(&ctx).await?;

    // Make a request to "some_echoer_service" API
    let route = route![some_secure_channel.clone(), "some_echoer_service"];
    let reply = ctx
        .send_and_receive::<_, _, String>(route, "Hello Ockam!".to_string())
        .await?;
    println!("Service worker returned: {}\n", reply);

    // Make a request to "some_other_echoer_service" API
    let route = route![some_secure_channel, "some_other_echoer_service"];
    let reply = ctx
        .send_and_receive::<_, _, String>(route, "Hello Other Ockam!".to_string())
        .await?;
    println!("Other service worker returned: {}\n", reply);

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}

// - A make-believe API service Worker ----------------------------------------

struct EchoerServiceWorker {
    abac_authorization: Arc<dyn AbacAuthorization>,
}

impl EchoerServiceWorker {
    pub fn new(abac_authorization: Arc<dyn AbacAuthorization>) -> Self {
        Self { abac_authorization }
    }
}

#[ockam::worker]
impl Worker for EchoerServiceWorker {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        // Echo the message body back on its return_route.
        ctx.send(msg.return_route(), msg.body()).await
    }

    async fn is_authorized(
        &mut self,
        _ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<bool> {
        // Grab the customer's IdentityIdentifer from
        // IdentitySecureChannelLocalInfo or any other place you may
        // have it lying around.
        let local_msg = msg.into_local_message();
        let local_info = IdentitySecureChannelLocalInfo::find_info(&local_msg)?;
        let identity_identifier = local_info.their_identity_id();

        // Create subject, resource, action
        let subject = Subject::from(identity_identifier.clone());
        let mut resource = Resource::from("/echoer");
        let action = Action::from(Method::Post);

        // Optional: Enrich attributes for any of Subject, Resource, Action.
        resource.extend([("space".into(), abac::string("some_customer_space"))]);

        // Perform the ABAC authorization request.
        self.abac_authorization
            .is_authorized(&subject, &resource, &action)
            .await
    }
}

// - An example in-place ABAC authorization implementation --------------------

pub struct SomeAbacAuthorizationImplementation {
    some_interface_to_authenticated_table: Arc<dyn AbacAttributeStorage>,
    some_interface_to_policy_storage: Arc<dyn AbacPolicyStorage>,
}

impl SomeAbacAuthorizationImplementation {
    pub fn new<A, P>(attribute_storage: A, policy_storage: P) -> Self
    where
        A: AbacAttributeStorage,
        P: AbacPolicyStorage,
    {
        Self {
            some_interface_to_authenticated_table: Arc::new(attribute_storage),
            some_interface_to_policy_storage: Arc::new(policy_storage),
        }
    }
}

#[async_trait]
impl AbacAuthorization for SomeAbacAuthorizationImplementation {
    async fn is_authorized(
        &self,
        subject: &Subject,
        resource: &Resource,
        action: &Action,
    ) -> Result<bool> {
        println!(
            "\nAbacAuthorization performing authorization for: {:?}, {:?}, {:?}",
            subject, resource, action
        );

        // Optional: enrich subject attributes from some authenticated table
        let attributes = self
            .some_interface_to_authenticated_table
            .get_subject_attributes(subject)
            .await?;
        let subject = subject.clone().with_attributes(attributes);

        println!("\nAbacAuthorization enriched subject: {:?}", subject);

        // retrieve the policy for resource, action and evaluate it
        match self
            .some_interface_to_policy_storage
            .get_policy(resource, action)
            .await?
        {
            Some(policy) => {
                println!("\nAbacAuthorizationWorker applying policy: {:?}\n", policy);
                Ok(policy.evaluate(&subject, resource, action))
            }
            None => {
                println!("\nAbacAuthorization failed to resolve policy");
                ockam::deny()
            }
        }
    }
}

// - A mock ockam cloud environment with the kind of services we need ---------

async fn mock_ockam_cloud(ctx: &Context) -> Result<Address> {
    // Create an identity for customer
    let customer_vault = Vault::create();
    let customer = Identities::create(ctx, &customer_vault).await?;
    let customer_identity = customer.identifier()?;

    // Create some abac implementation
    let some_interface_to_authenticated_table =
        fixtures::with_attribute_test_data(abac::mem::Memory::new(), customer_identity).await?;
    let some_interface_to_policy_storage =
        fixtures::with_policy_test_data(abac::mem::Memory::new()).await?;
    let abac_authorization = Arc::new(SomeAbacAuthorizationImplementation::new(
        some_interface_to_authenticated_table,
        some_interface_to_policy_storage,
    ));

    // Start some API service worker
    let worker = EchoerServiceWorker::new(abac_authorization.clone());
    WorkerBuilder::without_access_control("some_echoer_service", worker)
        .start(ctx)
        .await?;

    // Start some other API service worker
    let worker = EchoerServiceWorker::new(abac_authorization);
    WorkerBuilder::without_access_control("some_other_echoer_service", worker)
        .start(ctx)
        .await?;

    // Set up some identity secure channel for API service worker side
    let ockam_vault = Vault::create();
    let ockam = Identities::create(ctx, &ockam_vault).await?;
    let ockam_storage = InMemoryStorage::new();
    ockam
        .create_secure_channel_listener("ockam_listener", TrustEveryonePolicy, &ockam_storage)
        .await?;

    // Set up some identity secure channel for Customer request side
    let customer_storage = InMemoryStorage::new();
    let channel = customer
        .create_secure_channel("ockam_listener", TrustEveryonePolicy, &customer_storage)
        .await?;

    Ok(channel)
}
