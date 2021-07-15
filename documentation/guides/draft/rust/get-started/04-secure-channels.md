# Step 4 - Secure Channels

Secure channels are encrypted bi-directional message routes between two Entities. Secure channels consist of two peers running the secure channel protocol: a listener and an initiator.

## Creating a Secure Channel

Entities create a secure channel by calling `Entity::create_secure_channel_listener` on the listening peer, and `Entity::create_secure_channel` on the initiating peer.

```rust
// Listener  
let mut bob = Entity::create(&ctx)?;  
bob.create_secure_channel_listener("bob_secure_channel")?;  
  
// Initiator  
let mut alice = Entity::create(&ctx)?; 

// The channel address returned is the address for Alice to use
let channel = alice.create_secure_channel("bob_secure_channel")?;
```


When a secure channel is created, an address for the channel is returned. This address is used to route messages across the secure channel. 

## Example: Echoer through Secure Channel

In this example, Alice creates a secure channel with Bob. Alice and Bob are on different nodes, and use the TCP transport to route messages.


### Example: Bob (Listener)

```rust
      
use ockam::{Context, Entity, Result, Routed, SecureChannels, TcpTransport, Worker};

pub struct Echoer;

#[ockam::worker]
impl Worker for Echoer {
 type Message = String;
 type Context = Context;

 async fn handle_message(
     &mut self,
     ctx: &mut Self::Context,
     msg: Routed<Self::Message>,
 ) -> Result<()> {
     ctx.send(msg.return_route().clone(), msg.body()).await
 }

}

  

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {

 // Listener
 let mut bob = Entity::create(&ctx)?;
 bob.create_secure_channel_listener("bob_secure_channel")?;

 let tcp = TcpTransport::create(&ctx).await?;
 tcp.listen("127.0.0.1:4000").await?;
 ctx.start_worker("echoer", Echoer).await
}
```

### Example: Alice (Initiator)

```rust      
use ockam::{route, Context, Entity, Identity, Result, SecureChannels, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
 let tcp = TcpTransport::create(&ctx).await?;
 let bob_node = "127.0.0.1:4000";

 tcp.connect(bob_node).await?;

 let mut alice = Entity::create(&ctx)?;

 let channel = alice.create_secure_channel(route![(TCP, bob_node), "bob_secure_channel"])?;

 ctx.send(route![channel, "echoer"], "Hello, world!".to_string())
 .await?;
 
 let message = ctx.receive::<String>().await?;
 println!("{}", message);

 ctx.stop().await
}
```