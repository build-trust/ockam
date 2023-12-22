# Encrypted Portals between Macs – built in Swift and Rust

- [Portals for Mac](#portals-for-mac)
- [Introduction to Ockam Portals](#introduction-to-ockam-portals)
- [How we built a Swift macOS app that uses our Rust library](#how-we-built-a-swift-macos-app-that-uses-our-rust-library)

## Portals for Mac

Portals is a Mac app built in Swift. It uses the Ockam Rust library to privately share a service on your Mac with anyone, anywhere. The service is shared securely over an end-to-end encrypted Ockam Portal. Your friends can access it on their **localhost**!

<img width="1012" src="https://github.com/build-trust/ockam/assets/159583/5efbcbfa-5083-4682-941c-71d9a6c24f84">

Let's see the Portals app in action in this quick 2 minute video (please unmute for an explanation of what is happening):

https://github.com/build-trust/ockam/assets/159583/6e883e57-65c3-46d2-a05a-41fab4299c71

### Install Portals.app

If you use [Homebrew](https://brew.sh/) then:

```
brew update && brew install --cask build-trust/ockam/portals
```

Or, use the appropriate dmg file for your Mac:
- Apple Silicon based Mac: Install from [this dmg file](https://github.com/build-trust/ockam/releases/download/ockam_v0.113.0/ockam.app.aarch64-apple-darwin.dmg).
- Intel based Mac: Install from [this dmg file](https://github.com/build-trust/ockam/releases/download/ockam_v0.113.0/ockam.app.x86_64-apple-darwin.dmg).

## Introduction to Ockam Portals

An Ockam Portal carries a non-Ockam protocol over Ockam. There are various types of Portals: TCP Portals, UDP Portals, Kafka Portals, etc. The Portals app for Mac supports TCP Portals.

Ockam Portals are built on top of [Ockam Routing](https://docs.ockam.io/reference/protocols/routing), and end-to-end encrypted Ockam [Secure Channels](https://docs.ockam.io/reference/protocols/secure-channels). Channels can be established over multi-hop, multi-protocol transport routes that may include bridges, relays, or rendezvous. This layering enables end-to-end encrypted Portals. It also allows Portals to traverse NATs, firewalls, and clouds without any change to networks or infrastructure.

Under the covers, Ockam Secure Channels use lightweight and robust cryptographic primitives that have been proven, at scale, within modern systems like Signal, Whatsapp, and Wiregaurd. The design of Ockam's open source stack of protocols, and their use for TCP Portals, was audited by the security research firm Trail of Bits. They identified no immediate risks to the confidentiality or integrity of application data moving through Ockam. [A link to the final report will appear here when it's released in January]

A TCP Portal carries TCP over Ockam. It makes a remote TCP service appear on localhost; virtually adjacent to a local TCP client.

<img width="1012" src="https://github.com/build-trust/ockam/assets/159583/44bdfae0-fdb7-404f-8e2d-f08981c19076">

TCP Portals are different from VPNs because there is no virtual IP network. There is only a single, virtualized point-to-point TCP connection over an end-to-end encrypted channel. TCP Portals are also different from reverse proxies and load balancers because there is no exposed entrypoint on the Internet. The two ends of an Ockam Portal can live in completely private networks, that don't expose any listening ports, or allow any ingress at the network layer. Unlike TLS termination at loadbalancers, end-to-end Ockam Secure Channels do not expose your application's data to a third party operator. Data authenticity, integrity, and confidentiality are guaranteed between the two ends - your TCP service and its clients.

Each TCP Portal has two parts:
1. A TCP Outlet that runs adjacent to a TCP server. The Outlet decrypts and unwraps all Ockam Routing metadata and delivers raw TCP messages to the server. It also encrypts and wraps messages destined for clients with Ockam Routing metadata which allows these messages to be delivered to the corresponding Inlets.
2. A TCP Inlet that runs adjacent to one or more TCP clients. The Inlet encrypts and wraps any messages destined for the server in Ockam Routing metadata which allows these messages to be delivered to the corresponding Outlets. The Inlet also decrypts and unwraps all Ockam Routing metadata and delivers raw TCP messages to the clients.

If the Outlet is within a private network, the Outlet and Inlet nodes only make outgoing TCP connections, and the Outlet is made reachable to Inlets using an encrypted relay or a NAT puncturing rendezvous. An Inlet and an Outlet are mutually authenticated using unique cryptographic [identities and credentials](https://docs.ockam.io/reference/protocols/identities). Each connection is also authorized using granular [attribute-based access control](https://docs.ockam.io/reference/protocols/access-controls) policies.

<img width="1012" src="https://github.com/build-trust/ockam/assets/159583/cb40efe5-001d-4c04-aba0-9530e163abf2">

## How we built a Swift macOS app that uses our Rust library

The functionality of the Portals app was already implemented in the Ockam Rust library. All we had to do was create a great macOS-native experience.

Our first attempt at building the app was using Tauri. This made sense as we wanted to use the Ockam rust library and most people on our team are comfortable building things in Rust. This first version was easy to build and had all the basic functions we wanted. However, the experience of using the app wasn't great. Tauri only gave us minimal control over how the menu was rendered and what happened when a user interacts with the menu. This version of the app felt like it belonged in a 10 year old version of macOS when compared to super easy to use menubar items built into macOS Sonoma.

We realized that to have the rich experience we want, we must build the app using SwiftUI.

Unfortunately, we couldn't find an off-the-shelf solution, to integrate Swift and Rust, that would give us the best of both worlds; the safety of Rust, and the rich macOS-native experience of SwiftUI. After some more digging we realized we can connect the two using C-89. Rust is compatible with the C calling convention, and Swift is interoperable with Objective-C, which is a superset of C-89.

<img width="1012" src="https://github.com/build-trust/ockam/assets/159583/b5e691bd-fd96-41f0-922a-d32d7bf12f34">

We wrote the Rust data structures that needed to be visible to Swift twice. One version is idiomatic in Rust and easy to use. The other version is C compatible using pointers and memory that is manually allocated with `malloc`. We also exposed some C-compatible APIs that use raw-pointers in unsafe rust to convert the idiomatic data structures to their C-compatible versions. Finally we automatically generated a C header with the help of the `cbindgen` library.

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
