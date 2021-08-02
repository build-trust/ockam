# End-to-End Secure Communication for Distributed Applications

In this step-by-step guide we’ll learn how to build mutually-authenticated, end-to-end encrypted,
secure messaging channels that protect en-route messages against eavesdropping, tampering, and forgery.

Data, within modern distributed applications, are rarely exchanged over a single point-to-point
transport connection. Application messages routinely flow over complex, multi-hop, multi-protocol
routes — _across data centers, through queues and caches, via gateways and brokers_ — before reaching
their end destination.

Transport layer security protocols are unable to protect application messages because their protection
is constrained by the length and duration of the underlying transport connection. Ockam is a collection of
programming libraries (in Rust and Elixir) that make it simple for our applications to guarantee end-to-end
integrity, authenticity, and confidentiality of data.

We no longer have to implicitly depend on the defenses of every machine or application within the same,
usually porous, network boundary. Our application's messages don't have to be vulnerable at every point,
along their journey, where a transport connection terminates.

Instead, our application can have a strikingly smaller vulnerability surface and easily make
_granular authorization decisions about all incoming information and commands._

Let's build mutually-authenticated, end-to-end protected communication between distributed applications:

## Step-by-step

<ul>
<li><a href="./get-started/00-setup">00. Setup</a></li>
<li><a href="./get-started/01-node">01. Node</a></li>
<li><a href="./get-started/02-worker">02. Worker</a>
<li><a href="./get-started/03-routing">03. Routing</a></li>
<li><a href="./get-started/04-routing-many-hops">04. Routing over many hops</a></li>
<li><a href="./get-started/05-secure-channel">05. Secure Channel</a></li>
<li><a href="./get-started/06-secure-channel-many-hops">06. Secure Channel over many hops</a></li>
<li><a href="./get-started/07-routing-over-transport">07. Routing over a transport</a></li>
<li><a href="./get-started/08-routing-over-many-transport-hops">08. Routing over many transport hops</a></li>
<li>
<a href="./get-started/09-secure-channel-over-many-transport-hops">09. Secure Channel over many transport hops</a>
</li>
<li>
<a href="./get-started/10-secure-channel-with-entity">10. Secure Channel with Entity</a>
</li>
</ul>
