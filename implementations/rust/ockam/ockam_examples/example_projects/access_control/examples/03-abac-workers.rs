use abac_examples::{
    fixtures, AbacAuthorizationRequest, AbacAuthorizationResponse, AbacAuthorizationWorker,
    AbacPolicyWorker, AuthenticatedTableWorker,
};

use ockam::abac::{self, Action, Method, Resource, Subject};
use ockam::{
    authenticated_storage::InMemoryStorage,
    identity::{Identity, IdentitySecureChannelLocalInfo, TrustEveryonePolicy},
    vault::Vault,
};
use ockam::{route, Address, Context, Result, Routed, Worker, WorkerBuilder};

// - e.g. Some ockam_api implementation ---------------------------------------

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Get a-hold of some secure channel
    let some_secure_channel = mock_ockam_cloud(&ctx).await?;

    // Make a request to the "echoer" API's service worker.
    let route = route![some_secure_channel, "some_echoer_service"];
    ctx.send(route, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("Service worker returned: {}\n", reply);

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}

// - A make-believe API service Worker ----------------------------------------

struct EchoerServiceWorker {
    abac_authorization_address: Address,
}

impl EchoerServiceWorker {
    pub fn new<A: Into<Address>>(address: A) -> Self {
        Self {
            abac_authorization_address: address.into(),
        }
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
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<bool> {
        // Grab the customer's IdentityIdentifer from
        // IdentitySecureChannelLocalInfo or any other place you may
        // have it lying around.
        let local_msg = msg.into_local_message();
        let local_info = IdentitySecureChannelLocalInfo::find_info(&local_msg)?;
        let identity_identifier = local_info.their_identity_id();

        // Create an abac::Subject from the customer's IdentityIdentifier.
        let subject = Subject::from(identity_identifier.clone());

        // Create an abac::Resource, abac::Action that represents
        // whatever specific API call you need to authorize.
        let mut resource = Resource::from("/echoer");
        let action = Action::from(Method::Post);

        // Optional: Enrich attributes for any of Subject, Resource, Action.
        resource.extend([("space".into(), abac::string("some_customer_space"))]);

        // Perform the ABAC authorization request.
        let AbacAuthorizationResponse(decision) = ctx
            .send_and_receive(
                self.abac_authorization_address.clone(),
                AbacAuthorizationRequest(subject, resource, action),
            )
            .await?;

        Ok(decision)
    }
}

// - A mock ockam cloud environment with the kind of services we need ---------

async fn mock_ockam_cloud(ctx: &Context) -> Result<Address> {
    // Create an identity for customer
    let customer_vault = Vault::create();
    let customer = Identities::create(ctx, &customer_vault).await?;
    let customer_identity = customer.identifier()?;

    // Start some ABAC policy source
    let backend = fixtures::with_policy_test_data(abac::mem::Memory::new()).await?;
    let worker = AbacPolicyWorker::new(backend);
    WorkerBuilder::without_access_control("some_abac_policy_source", worker)
        .start(ctx)
        .await?;

    // Start some authenticated table service
    let backend =
        fixtures::with_attribute_test_data(abac::mem::Memory::new(), customer_identity).await?;
    let worker = AuthenticatedTableWorker::new(backend);
    WorkerBuilder::without_access_control("some_authenticated_table_service", worker)
        .start(ctx)
        .await?;

    // Start some ABAC authorization decision point
    let worker = AbacAuthorizationWorker::new(
        "some_authenticated_table_service",
        "some_abac_policy_source",
    );
    WorkerBuilder::without_access_control("some_abac_decision_point", worker)
        .start(ctx)
        .await?;

    // Start some API service worker
    WorkerBuilder::without_access_control(
        "some_echoer_service",
        EchoerServiceWorker::new("some_abac_decision_point"),
    )
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
