# Encrypted Portals between Macs – built in Swift and Rust

- [Portals for Mac](#portals-for-mac)
- [Introduction to Ockam Portals](#introduction-to-ockam-portals)
- [How we built a Swift macOS app that uses our Rust library](#how-we-built-a-swift-macos-app-that-uses-our-rust-library)
- [Step-by-Step: How an End-to-End Encrypted Portal is established](#step-by-step-how-an-end-to-end-encrypted-portal-is-established)
- [Open Source: Sponsorship Matching Program](#sponsorship-matching-program)

## Portals for Mac

Portals is a Mac app built in Swift. It uses the Ockam Rust library to privately share a service on your Mac to anyone, anywhere. The service is shared securely over an end-to-end encrypted Ockam Portal. Your friends will have access to it on their **localhost**!

<img width="1012" src="https://github.com/build-trust/ockam/assets/159583/5efbcbfa-5083-4682-941c-71d9a6c24f84">

Let's see Portals in action in this quick 2 minute video (please unmute for an explanation of what is happening):

https://github.com/build-trust/ockam/assets/159583/6e883e57-65c3-46d2-a05a-41fab4299c71

### Install

If you use [Homebrew](https://brew.sh/):

```
brew update && brew install build-trust/ockam/portals
```

Or, install it using the appropriate dmg file for your Mac:
- Apple Silicon based Mac: Install from [this dmg file](https://github.com/build-trust/ockam/releases/download/ockam_v0.115.0/ockam.app.aarch64-apple-darwin.dmg).
- Intel based Mac: Install from [this dmg file](https://github.com/build-trust/ockam/releases/download/ockam_v0.115.0/ockam.app.x86_64-apple-darwin.dmg).

### Ideas for things you can use it for

- Share an in-dev Nextjs or Svelte app with a teammate.
- Create a static file server with Caddy Server and share with a friend.
- Share a dev Kubernetes cluster using Kind or Minikube with team mates.
- Share Visual Studio Code between your computers and access it on the go using CoderHQ's code-server.
- SSH to a home computer when you're on the move.
- Safely browse the Internet from anywhere in the world using a SOCKS proxy to your home computer.
- Transfer files between your computers over rsync or sftp.
- Share a Postgres database with a teammate.
- Share a Jupyter analytics notebook with a teammate.

The possibilities are endless! If you have other ideas or need help setting up any of the above, [join us on discord](https://discord.ockam.io).

## Introduction to Ockam Portals

An Ockam Portal carries a non-Ockam protocol over Ockam. There are various types of Ockam Portals: TCP Portals, UDP Portals, Kafka Portals, etc. The Portals app for Mac supports TCP Portals.

Ockam Portals are built on top of [Ockam Routing](https://docs.ockam.io/reference/protocols/routing), and end-to-end encrypted Ockam [Secure Channels](https://docs.ockam.io/reference/protocols/secure-channels). Channels can be established over multi-hop, multi-protocol transport routes that may include bridges, relays, or rendezvous. This layering enables end-to-end encrypted Portals. It also allows Portals to traverse NATs, firewalls, and clouds without any change to networks or infrastructure.

Under the covers, Ockam Secure Channels use lightweight cryptographic primitives that have been proven, at scale, within modern systems like Signal and Wiregaurd. _The design of Ockam's open source stack of protocols, and their use for TCP Portals, was audited by the security research firm Trail of Bits._ They highlighted that Ockam’s protocols use robust cryptography and identified no risks to the confidentiality or integrity of application data moving through Ockam. [A link to the final report will appear here when it's released in January]

A TCP Portal carries TCP over Ockam. It makes a remote TCP service appear on localhost; virtually adjacent to a local TCP client.

<img width="1012" src="https://github.com/build-trust/ockam/assets/159583/44bdfae0-fdb7-404f-8e2d-f08981c19076">

TCP Portals are different from VPNs because there is no virtual IP network. There is only a single, virtualized point-to-point TCP connection over an end-to-end encrypted channel. TCP Portals are also different from reverse proxies and load balancers because there is no exposed entrypoint on the Internet. The two ends of an Ockam Portal can live in completely private networks, that don't expose any listening ports, or allow any ingress at the network layer. Unlike TLS termination at load balancers, end-to-end Ockam Secure Channels do not expose your application's data to a third party operator. Data authenticity, integrity, and confidentiality are guaranteed between the two ends - your TCP service and its clients.

Each TCP Portal has two parts:
1. A TCP Outlet that runs adjacent to a TCP server. The Outlet decrypts and unwraps all Ockam Routing metadata and delivers raw TCP messages to the server. It also encrypts and wraps messages destined for clients with Ockam Routing metadata which allows these messages to be delivered to the corresponding Inlets.
2. A TCP Inlet that runs adjacent to one or more TCP clients. The Inlet encrypts and wraps any messages destined for the server in Ockam Routing metadata which allows these messages to be delivered to the corresponding Outlets. The Inlet also decrypts and unwraps all Ockam Routing metadata and delivers raw TCP messages to the clients.

If the Outlet is within a private network, the Outlet and Inlet nodes only make outgoing TCP connections, and the Outlet is made reachable to Inlets using an encrypted relay or a NAT puncturing rendezvous. An Inlet and an Outlet are mutually authenticated using unique cryptographic [identities and credentials](https://docs.ockam.io/reference/protocols/identities). Each connection is also authorized using granular [attribute-based access control](https://docs.ockam.io/reference/protocols/access-controls) policies.

<img width="1012" src="https://github.com/build-trust/ockam/assets/159583/cb40efe5-001d-4c04-aba0-9530e163abf2">

## How we built a Swift macOS app that uses our Rust library

The Portals functionality was already implemented in the Ockam Rust library. We set out to create a great macOS-native experience.

Our first attempt at building the app was using Tauri. This made sense as we wanted to use the Ockam Rust library and most people on our team are comfortable building things in Rust. This first version was easy to build and had all the basic functions we wanted. However, the experience of using the app wasn't great. Tauri only gave us minimal control over how the menu was rendered and what happened when a user interacts with the menu. This version of the app felt like it belonged in a 10 year old version of macOS when compared to super easy to use menubar items built into macOS Sonoma.

We realized that to have the rich experience we want, we must build the app using SwiftUI.

Unfortunately, we couldn't find an off-the-shelf solution, to integrate Swift and Rust, that would give us the best of both worlds; the safety of Rust, and the rich macOS-native experience of SwiftUI. After some more digging we realized we can connect the two using C-89. Rust is compatible with the C calling convention, and Swift is interoperable with Objective-C, which is a superset of C-89.

<img width="1012" src="https://github.com/build-trust/ockam/assets/159583/b5e691bd-fd96-41f0-922a-d32d7bf12f34">

We wrote the Rust data structures that needed to be visible to Swift twice. One version is idiomatic in Rust and easy to use. The other version is C compatible using pointers and memory that is manually allocated with `malloc`. We also exposed some C-compatible APIs that use raw-pointers in unsafe Rust to convert the idiomatic data structures to their C-compatible versions. Finally we automatically generated a C header with the help of the `cbindgen` library.

On the Swift side, we could have called the C APIs directly, but C data structures are not first class citizens in Swift. This makes them harder to use idiomatically within SwiftUI code. Instead, we chose to duplicate the data structures in Swift and convert between C and Swift. This may seem burdensome, but practically, the shared state doesn't change very often. The ability to quickly build components in SwiftUI using constructs like `if let ..`, `ForEach`, `enum` etc. is super helpful and worth the tradeoff.

Here's an example of the same structure in its 4 forms:

```
// Rust idiomatic structure
#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct LocalService {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub shared_with: Vec<Invitee>,
    pub available: bool,
}

// Rust C-compatible structure
#[repr(C)]
pub struct LocalService {
    pub(super) name: *const c_char,
    pub(super) address: *const c_char,
    pub(super) port: u16,
    pub(super) shared_with: *const *const Invitee,
    pub(super) available: u8,
}

// Generated C header structure
typedef struct C_LocalService {
  const char *name;
  const char *address;
  uint16_t port;
  const struct C_Invitee *const *shared_with;
  uint8_t available;
} C_LocalService;

// Swift idiomatic structure
class LocalService {
    let name: String
    @Published var address: String?
    @Published var port: UInt16
    @Published var sharedWith: [Invitee]
    @Published var available: Bool
}
```

The Swift app is statically linked to our Rust lib at compile time. The data flow is simple: UI interactions are sent from Swift to Rust as actions by calling C APIs, change events are emitted only by Rust, and Swift is notified using callbacks that lead to updates to the UI.

Most code in the SwiftUI views looks just like any other SwiftUI application.

```swift
VStack(alignment: .leading, spacing: 0) {
    Text(service.sourceName).lineLimit(1)

    HStack(spacing: 0) {
        Image(systemName: "circle.fill")
            .font(.system(size: 7))
            .foregroundColor( service.enabled ? (service.available ? .green : .red) : .orange)

        if !service.enabled {
            Text(verbatim: "Not connected")
        } else {
            if service.available {
                Text(verbatim: service.address.unsafelyUnwrapped + ":" + String(service.port))
            } else {
                Text(verbatim: "Connecting")
            }
        }
    }
...
```

If you're curious to learn more, checkout the code for the [ockam_app_lib crate](https://github.com/build-trust/ockam/tree/develop/implementations/rust/ockam/ockam_app_lib) and the Portals [app in Swift](https://github.com/build-trust/ockam/tree/develop/implementations/swift/ockam/ockam_app). The [Makefile](https://github.com/build-trust/ockam/blob/develop/implementations/swift/Makefile) in the swift folder is also a good place to explore how everything is built and linked together.

If you're interested in contributing to the app's Swift or Rust code, we add new [good first issues](https://github.com/build-trust/ockam/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) every week and love helping new contributors. Join us on the [contributors discord](https://discord.ockam.io/).

## Step-by-Step: How an End-to-End Encrypted Portal is established

Let's take a step by step look at how a TCP service, running on your Mac, becomes securely accessible on localhost, on your friend's Mac.

1. On first launch, the Portals app, asks you to Enroll with Ockam Orchestrator. You click the Enroll button and it starts the _OAuth 2.0 Authorization Code Flow with PKCE_ with `https://account.ockam.io`. You then signup / login with your GitHub username or email address. When the OAuth flow completes, the app receives an `access_token`. It use that token to query your account information from `https://account.ockam.io`. During this process we may send you an email to verify your email address.

1. The app generates a unique [Ockam Identity](https://docs.ockam.io/reference/protocols/identities#identities), with its secret keys in an [Ockam Vault](https://docs.ockam.io/reference/protocols/keys) stored on disk. Ockam Identities are cryptographically verifiable digital identities. Each Identity maintains one or more secret keys and has a unique Identifier. It's possible to use Vaults that store secrets in Apple Keychain, Secure Enclave, Yubikey, 1Password etc. If that's interesting, [please tell us](https://discord.ockam.io) so we know which ones to prioritize in our plans. Vaults are pluggable and really simple to build. Ockam Vault in [AWS KMS](https://github.com/build-trust/ockam/tree/develop/implementations/rust/ockam/ockam_vault_aws) is a good example to learn how to build one. We'd love to [help you](https://discord.ockam.io/) contribute a [Vault](https://docs.ockam.io/reference/protocols/keys) implementation for you favorite place to store secrets.

1. Next, the app establishes a mutually authenticated [Secure Channel](https://docs.ockam.io/reference/protocols/secure-channels) with Ockam Orchestrator's Controller. At compile-time, the app, was hardcoded with the Ockam [Identifier](https://docs.ockam.io/reference/protocols/identities#identities) and [Route](https://docs.ockam.io/reference/protocols/routing) to the Controller. The app sends the OAuth `access_token` to the Controller over this secure channel. The Controller uses that token to independently query your account information from `https://account.ockam.io`.

1. Based on who you are, Ockam Orchestrator's Controller decides which Spaces and Projects you may access. All Portals for Mac accounts get _a dedicated Space with a free Community Edition subscription_. This free subscription allows you to create [Relays](https://docs.ockam.io/reference/protocols/routing#relay) and a [Credential Authority](https://docs.ockam.io/reference/protocols/identities#credentials) with some limits on the amount of encrypted data that you may relay and the number people who can access your Portals. If your dedicated Space and Project have not been provisioned so far, the Controller provisions them and returns the Identifier and Route to your project to the Portals app over the previously established Secure Channel.

1. The Portals app then creates an outgoing TCP connection and a Secure Channel over that connection with your dedicated Project Node in the cloud. It requests the Relay service to create an encrypted Relay for your app at the [Project Node](https://docs.ockam.io/reference/protocols/nodes). The Relay remembers the route to the Portals app node on your computer and any messages delivered to this Relay address would be routed to your app.

1. You then click _"Open a new Portal Outlet.."_ to a TCP service. The Outlet remembers the address of the service and waits for messages from Inlets. The Outlet doesn't expose any listening ports. Your service can remain completely local to your machine. The Outlet also enforces an [access control policy](https://docs.ockam.io/reference/protocols/access-controls) that requires that requests to establish a portal must come from an Identity that is a member of your Project and associated with an invited email address. Requests from Inlets must present a Credential from your Project's Credential Authority attesting to an Identity's email address and membership status.

1. Next, you invite a friend to the TCP Portal. The Portals app adds their email address to the Outlet's access control. It then talks to your Project's dedicated Credential Authority to generate an Enrollment Ticket that your friend's app can use to become a member of your Project. The Invitation includes your email, the Enrollment Ticket, and a Route to your Outlet through your Relay. The app stores the Invitation with the Orchestrator's Controller which sends a notification to the invited email address.

1. Your invited friend experiences steps 1-5 above. They generate their own Identity and get a dedicated Orchestrator Space. Their app fetches all their Invitations from the Orchestrator's Controller. Included in that list is the Invitation from you. If they accept this Invitation, their Portals app uses the included Enrollment Ticket to create a Secure Channel with your Project's Credential Authority and exchange it for a Credential that ties their Identity with their verified email address and project membership.

1. Your friend's app, its Ockam Identity, is from this point a member of your Project in Ockam Orchestrator. It knows the Route to your Outlet through your Relay in you Project Node. It also possesses a Credential to authenticate their attributes (email address and project membership) to the Project Node and to your Portals app Node. Their app creates an outgoing TCP connection with your Project Node and sets up a authenticated secure channel to get access to your Relay. It then creates an end-to-end encrypted, mutually authenticated secure channel, via your Relay to your Portals app.

1. Finally, your friend's app starts a TCP Inlet listener on `127.0.0.1`. The Inlet [Worker](https://docs.ockam.io/reference/protocols/nodes) in _their_ app runs the portal protocol with the Outlet worker in _your_ app. This Portal is established over the end-to-end encrypted Secure Channel. The Inlet enforces an access control policy that requires that the Outlet must present an Identity that is a member of your Project and associated with the email address that sent the invitation. TCP segments received at the Inlet's TCP listener are wrapped in Ockam Routing messages and sent through the end-to-end Secure Channel which encrypts them. In your app node, at the other end of the secure channel, the messages are decrypted and routed to the Outlet. All routing information is removed and raw TCP segments are sent to your TCP service.

#### NAT traversal

In the steps above you'll notice that neither ends of the Portal expose any listening TCP servers on the Internet. Both sides make an outgoing TCP connection to the Node that offers the Relay service. This approach allows Portals to traverse NATs without any change to networks or infrastructure. Relays are highly reliable and always work if the two private networks allow outgoing connections to the Internet. This reliability, however, comes with a tradeoff - end-to-end encrypted data must travel through the relay.

[Ockam Routing](https://docs.ockam.io/reference/protocols/routing) allows us to establish end-to-end secure channels over all sorts of routing topologies. An alternative to Relays (which are simular to TURN) is to use a Rendezvous (which is similar to STUN). In this approach a Rendezvous service is used for puncturing through NATs. It has the advantage that the encrypted data no longer needs to travel through a relay. However, it also has the disadvantage of being unreliable. Puncturing through NATs doesn't always work because of the large variety of network routers and differences in how they implement network address translation. Various research papers suggest a success rate between 70 to 80 percent.

A good compromise, we think, would be to combine the two approaches. Try to use a Rendezvous and if it doesn't work failover to using a Relay. Here are a couple of examples of using Rendezvous with Ockam Routing: [in Rust](https://github.com/build-trust/ockam/blob/develop/implementations/rust/ockam/ockam_transport_udp/examples/puncher.rs), and [in Elixir](https://github.com/build-trust/ockam/pull/6588). The Portals app doesn't support Rendezvous yet. If you're interested in that functionality [please tell us](https://discord.ockam.io) about it so we know to prioritize it in our plans.

#### Verifying and Pinning Friend Identifiers

Another thing to note is that mutual trust, between the Portal Outlet and Inlet, relies on attestations by your Project's Credential Authority. This approach is very convenient but also presents a challenge. A compromised Authority could falsely attest to access control attributes and grant access to an unauthorized participant. This challenge is similar to why the Signal messenger recommends that you should out-of-band verify Signal's "Safety Numbers" with your friends. Without that verification, a compromise of Signal's servers may result in an attacker pretending to be someone you know. Similar to Signal's approach, we intend to add the ability to verify and pin friend Identifiers in the Portals app. If you're interested in that functionality [please tell us](https://discord.ockam.io) about it and stay tuned on our progress.

## Sponsorship Matching Program

Portals for Mac and the Ockam Orchestrator _Community Edition Subscription_ are free to use. If you like the app, please consider contributing to Ockam's [Sponsorship Matching Program](https://github.com/sponsors/build-trust). We sponsor open source builders who are making it possible for software to be more private and secure. If you sponsor the Ockam Open Source project, we will match your contribution and pass it along to other open source developers. For example, if you [sponsor Ockam](https://github.com/sponsors/build-trust) for $10 a month, we will match it and send $20 back out into the community. [👉](https://github.com/sponsors/build-trust)

---

_Give [Portals for Mac](#install) a try and [star the Ockam project](https://github.com/build-trust/ockam) so more people can learn about it._
