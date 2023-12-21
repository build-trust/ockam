# Encrypted Portals between Macs – built in Swift and Rust

- [What is a Portal and how you can use it.](#what-is-a-portal-and-how-you-can-use-it)
- [How we built a Swift macOS app that uses our Rust library.](#how-we-built-a-swift-macos-app-that-uses-our-rust-library)
- [The protocols that enable remote TCP service to appear on localhost next to a client.](#the-protocols-that-enable-remote-tcp-service-to-appear-on-localhost-next-to-a-client)

## What is a Portal and how you can use it

A TCP Portal makes a remote TCP service appear on localhost, virtually adjacent to TCP clients.

All communication happens over end-to-end encrypted and mutually authenticated Ockam Secure Channels. Channels are established over multi-hop, multi-protocol transport routes that can include bridges, relays, or rendezvous. This enables end-to-end encrypted portals that can traverse NATs, firewalls, and clouds without any change to networks or infrastructure.

TCP Portals are different from VPNs because there is no virtual IP network, there is only a single virtualized point-to-point TCP connection over an end-to-end encrypted channel. TCP Portals are also different from reverse proxies and load balancers because there is no exposed entrypoint to the Internet. The two ends of an Ockam Portal can live in completely private networks that don't expose any listening ports or allow any ingress. Unlike TLS termination at loadbalancers, end-to-end Ockam Secure Channels do not expose your application's data to a third party operator. Data authenticity, integrity, and confidentiality are guaranteed between the two ends.

Under the covers Ockam Secure Channels use lightweight and robust cryptographic primitives that have been proven, at scale, within modern systems like Signal, Whatsapp, and Wiregaurd. The design of our open source stack of [protocols](#the-protocols-that-enable-remote-tcp-service-to-appear-on-localhost-next-to-a-client) and their use for TCP Portals was recently audited by the security research firm Trail of Bits. The executive summary of their report states: _"None of the identified issues pose an immediate risk to the confidentiality and integrity of data handled by the system in the context of the two in-scope use cases (TCP Portals and Kafka Portals). The majority of identified issues relate to information that should be added to the design documentation."_ We're very excited about the report and in the coming weeks, we'll share this report and a detailed writeup about the audit process.

You can create production ready portals using Ockam Command to privately connect applications across companies, VPCs, regions, clouds, and data centers. Our team is fully remote and we wanted an easy GUI to privately share services with our teammates and friends. Everyone on our team uses a Mac so we created an open source macOS menubar app using SwiftUI and the Ockam Rust library.

<img width="1012" src="https://github.com/build-trust/ockam/assets/159583/5efbcbfa-5083-4682-941c-71d9a6c24f84">

Each TCP Portal has two parts:
1. A TCP Outlet runs adjacent to a TCP server. The outlet decrypts and unwraps all Ockam Routing information and delivers raw TCP messages to the server. It also encrypts and wraps messages destined for clients with Ockam Routing information which allows these messages to be delivered to the corresponding Inlets.
2. A TCP Inlet that runs adjacent to one or more TCP clients. The inlet encrypts and wraps any messages destined for the server in Ockam Routing information which allows these messages to be delivered to the corresponding Outlets. The inlet also decrypts and unwraps all Ockam Routing information and delivers raw TCP messages to the clients.

If the outlet is within a private network, the outlet and inlet nodes only make outgoing TCP connections and the outlet is made reachable to inlets using an encrypted relay or a NAT puncturing rendezvous. An inlet and an outlet are mutually authenticated using unique cryptographic identities and credentials. Each connection is also authorized using granular attribute-based access control policies.

<img width="1012" src="https://github.com/build-trust/ockam/assets/159583/cb40efe5-001d-4c04-aba0-9530e163abf2">

### Portals.app

First, let's install the app. If you use [Homebrew](https://brew.sh/), then you can install with this simple command:

```
brew update && brew install --cask build-trust/ockam/portals
```

If you prefer to install the app manually, download and install it using the appropriate dmg file for your Mac. If you have an Apple Silicon based Mac, install from [this dmg file](https://github.com/build-trust/ockam/releases/download/ockam_v0.113.0/ockam.app.aarch64-apple-darwin.dmg). If you have an Intel based Mac, install from [this dmg file](https://github.com/build-trust/ockam/releases/download/ockam_v0.113.0/ockam.app.x86_64-apple-darwin.dmg).

Next, let's see the Portals.app in action in this quick 2 minute video (please unmute for an explanation of what is happening):

https://github.com/build-trust/ockam/assets/159583/e07ebb8e-b6f6-436b-8f5c-d7e7c4c19e5e
