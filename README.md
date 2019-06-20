<h1 align="center">
	<img width="500" alt="Ockam" src="logo.png">
</h1>

<p align="center">
<a href="https://dev.azure.com/ockam-network/ockam/_build/latest?definitionId=10?branchName=master"><img alt="Apache 2.0 License" src="https://dev.azure.com/ockam-network/ockam/_apis/build/status/ockam-network.ockam?branchName=master"></a>
<a href="LICENSE"><img alt="Apache 2.0 License" src="https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=flat-square"></a>
<a href="https://godoc.org/github.com/ockam-network/ockam"><img alt="GoDoc" src="https://img.shields.io/badge/godoc-reference-blue.svg?style=flat-square"></a>
<a href="https://join.slack.com/t/ockam-community/shared_invite/enQtNDk5Nzk2NDA2NDcxLWMzMzJlZjQzOTZjYWY0YmNkNWE1NmI1M2YyYzlkNjk4NDYyYzU0OWE0YTI4ZjcwNDBjNmQ4NzZjZTMzYmY3NDA"><img alt="Discuss Ockam" src="https://img.shields.io/badge/slack-discuss-E01563.svg?logo=slack&style=flat-square"></a>
</p>

## Introduction

[Ockam](https://www.ockam.io) is a collection of tools to help you establish secure connections and trustful exchange of information within connected systems.

To understand the key ideas behind Ockam, please read our short concept papers of on:
1. [Why we're focusing on the connectivity and messaging layer?](document/concept/0001-secure-connectivity-and-messaging)
2. [What exactly does trust mean within connected systems?](document/concept/0002-trust-architecture)
3. [What are the minimum criteria for establishing trust?](document/concept/0003-minimum-criteria-for-trust)

Ockam's core features include:

### Secure Identity & Credential Management

* [Decentralized Identifiers](document/concept/0004-decentalized-identifiers): Cryptographically provable, decentralized identifiers (DIDs) for each device to ensure data integrity and protect against identity spoofing.

* [Device And Service Registry](document/concept/0005-entity-registry): Ockam Blockchain based registry for discovering public keys, protocols, endpoints and other metadata about a device or a service.

* [Hardware Vault And Cryptography](document/concept/0006-hardware-vault-and-cryptography): Safely store private keys and credentials in hardware and easily leverage cryptographic modules or enclaves to sign device or service generated data.

* [Key And Credential Management](document/concept/0007-key-and-credential-management): Easily setup, rotate, revoke keys and other credentials without complex and brittle PKI.

### Secure Connectivity & Messaging

* [Trustful Communication](document/concept/0008-trustful-communication): Use Verifiable Credentials and Peer-to-peer Mutual Authentication to establish trust: device-to-device or device-to-service.

* [End-To-End Encrypted Messaging](document/concept/0009-end-to-end-encrypted-messaging): Secure, efficient and scalable end-to-end encrypted messaging to protect against tampering, replay, snooping and man-in-the-middle attacks.

* [Trusted Twins](document/concept/0010-trusted-twins): Cloud based, persistent, mutually trusted twin of each device so applications can interact with device state and enqueue messages even when a device is offline.

* [Ockam Blockchain Network](document/concept/0011-blockchain-network): A fast finality, safety favouring, light client friendly, and horizontally scalable blockchain network that is optimized for connected devices.

## Install

This repository includes a Golang package that can be used to build Go programs (device firmware or a backend service) that act as light nodes on the Ockam Blockchain Network.

With Go version `1.11+` installed, add the ockam Golang package to your project using `go get`:
```
go get github.com/ockam-network/ockam
```

Once you have the `ockam` package, copy an example from the [example directory](example/) and run it using `go run`.

```
go run example/01_hello_ockam.go
```

## Contribute

- [Ask a question](CONTRIBUTING.md#ask-a-question)
- [Report an issue or a bug](CONTRIBUTING.md#report-an-issue-or-a-bug)
- [Share an idea for a new feature](CONTRIBUTING.md#share-an-idea-for-a-new-feature)
- [Contribute Code](CONTRIBUTING.md#contribute-code)
	- [Development Environment](CONTRIBUTING.md#development-environment)
	- [Build](CONTRIBUTING.md#build)
	- [Lint](CONTRIBUTING.md#lint)
	- [Test](CONTRIBUTING.md#test)
	- [Project Conventions](CONTRIBUTING.md#project-conventions)
		- [Spacing and Indentation](CONTRIBUTING.md#spacing-and-indentation)
		- [Code Format](CONTRIBUTING.md#code-format)
		- [Commit Messages](CONTRIBUTING.md#commit-messages)
		- [Git Workflow](CONTRIBUTING.md#git-workflow)
		- [Signed Commits](CONTRIBUTING.md#signed-commits)
	- [Error Handling](CONTRIBUTING.md#error-handling)
	- [Effective Go](CONTRIBUTING.md#effective-go)
	- [Send a Pull Request](CONTRIBUTING.md#send-a-pull-request)
- [Code of Conduct](CONTRIBUTING.md#code-of-conduct)

## Contributors

* [Brian Schroeder](https://github.com/bts)
* [Brett Nitschke](https://github.com/BrettNitschke)
* [Carlos Flores](https://github.com/carlosflrs)
* [Jeff Malnick](https://github.com/malnick)
* [Logan Jager](https://github.com/jagtek)
* [Matthew Gregory](https://github.com/mattgreg)
* [Mrinal Wadhwa](https://github.com/mrinalwadhwa)
* [Rolf Kaiser](https://github.com/rkaiser0324)

## License and attributions

This code is licensed under the terms of the [Apache License 2.0](LICENSE)

This code depends on other open source packages; attributions for those packages are in the [NOTICE](NOTICE) file.
