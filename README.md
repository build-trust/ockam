<h1 align="center">
	<img width="500" alt="Ockam" src="logo.png">
</h1>

<p align="center">
<a href="https://dev.azure.com/ockam-network/ockam/_build/latest?definitionId=10?branchName=master"><img alt="Apache 2.0 License" src="https://dev.azure.com/ockam-network/ockam/_apis/build/status/ockam-network.ockam?branchName=master"></a>
<a href="LICENSE"><img alt="Apache 2.0 License" src="https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=flat-square"></a>
<a href="https://godoc.org/github.com/ockam-network/ockam"><img alt="GoDoc" src="https://img.shields.io/badge/godoc-reference-blue.svg?style=flat-square"></a>
<a href="https://join.slack.com/t/ockam-community/shared_invite/enQtNDk5Nzk2NDA2NDcxLWMzMzJlZjQzOTZjYWY0YmNkNWE1NmI1M2YyYzlkNjk4NDYyYzU0OWE0YTI4ZjcwNDBjNmQ4NzZjZTMzYmY3NDA"><img alt="Discuss Ockam" src="https://img.shields.io/badge/slack-discuss-E01563.svg?logo=slack&style=flat-square"></a>
</p>

<h1 align="center">
	<img width="900" alt="ockam register" src="register.gif">
</h1>

## Overview

[Ockam](ockam.io) is a decentralized and open platform for easily adding identity, trust and interoperability
to connected devices.

This repository contains:
1. The `ockam` command line program for simple interactions with the Ockam Network.
2. The `github.com/ockam-network/ockam` Go package to develop Go applications that programatically interact with the
   Ockam Network.

In the near future, we plan to add `ockam` packages for other programming languages.

## Command Line

The simplest way to get started is to download the latest `ockam` command for your operating system. You can get it
from our [release bundles](https://github.com/ockam-network/ockam/releases) or using this simple
[script](godownloader-ockam.sh):

```
curl -L https://git.io/fhZgf | sh
```

This will download the command to `./bin/ockam` in your current directory. The binary is self contained, so if you
wish to you can copy it to somewhere more convenient in your system path, for example:

```
cp ./bin/ockam /usr/local/bin/
```

Once the command is in you path, you can run:

```
ockam --version
```

Next you may call:
```
ockam register
```
which will generate a unique ockam [decentralized identity](https://github.com/w3c-ccg/did-primer) for
your computer and register it on the Ockam TestNet.

## Go Package

You can add the ockam Go package to your project just like any other Go package, by calling `go get`:
```
go get github.com/ockam-network/ockam
```

We require Go version `1.11+`.

## Hello Ockam

Here is some simple Go code to connect with the Ockam TestNet:

```go
// create a lightweight local ockam node and give it a way to find peers on the ockam test network
ockamNode, err := node.New(node.PeerDiscoverer(http.Discoverer("test.ockam.network", 26657)))
if err != nil {
	log.Fatal(err)
}

// ask the local node to find peers and sync with network state
err = ockamNode.Sync()
if err != nil {
	log.Fatal(err)
}

// print the id of the chain that the network is maintaining.
ockamChain := ockamNode.Chain()
fmt.Printf("Chain ID: %s\n", ockamChain.ID())
```

*Note:* The Ockam Testnet is provided and maintained by the Ockam team to help you build and experiment with
applications that interact with Ockam. The TestNet has no service level gauruntees, may have intermittent availability,
may be down for maintenance, and may be restarted at anytime. If your application needs a production ready network,
please email the Ockam team at hello@ockam.io

## Register an Entity

```go
// create a new ed25519 signer
signer, err := ed25519.New()
if err != nil {
	log.Fatal(err)
}

// create a new ockam entity to represent a temperature sensor
temperatureSensor, err := entity.New(
	entity.Attributes{
		"name":         "Temperature Sensor",
		"manufacturer": "Element 14",
		"model":        "Raspberry Pi 3 Model B+",
	},
	entity.Signer(signer),
)
if err != nil {
	log.Fatal(err)
}

// register the entity by creating a signed registration claim
registrationClaim, err := ockamChain.Register(temperatureSensor)
if err != nil {
	log.Fatal(err)
}

fmt.Printf("registrationClaim - %s\n", registrationClaim.ID())
```

This [verifiable](https://www.w3.org/TR/verifiable-claims-data-model/) registration claim embeds the
[DID Document](https://w3c-ccg.github.io/did-spec/#dfn-did-document) that represents this newly created entity.

```
{
	"@context": [
		"https://w3id.org/identity/v1",
		"https://w3id.org/security/v1"
	],
	"id": "did:ockam:2QyqWz4xWB5o4Pr9G9fcZjXTE2ej5/claim/1brpf2pkh6",
	"type": [
		"EntityRegistrationClaim"
	],
	"issuer": "did:ockam:2QyqWz4xWB5o4Pr9G9fcZjXTE2ej5",
	"issued": "2019-01-10",
	"claim": {
		"authentication": [
			{
				"publicKey": "did:ockam:2QyqWz4xWB5o4Pr9G9fcZjXTE2ej5#key-1",
				"type": "Ed25519SignatureAuthentication2018"
			}
		],
		"id": "did:ockam:2QyqWz4xWB5o4Pr9G9fcZjXTE2ej5",
		"manufacturer": "Element 14",
		"model": "Raspberry Pi 3 Model B+",
		"name": "Temperature Sensor",
		"publicKey": [
			{
				"id": "did:ockam:2QyqWz4xWB5o4Pr9G9fcZjXTE2ej5#key-1",
				"publicKeyHex": "3c93f446990ecd3ce64bcf9a5f949423d2e348948ee3aeb1c78924490f6b50f9",
				"type": "Ed25519VerificationKey2018"
			}
		],
		"registrationClaim": "did:ockam:2QyqWz4xWB5o4Pr9G9fcZjXTE2ej5/claim/1brpf2pkh6"
	},
	"signatures": [
		{
			"created": "2019-01-10T07:53:25Z",
			"creator": "did:ockam:2QyqWz4xWB5o4Pr9G9fcZjXTE2ej5#key-1",
			"domain": "ockam",
			"nonce": "1brpf2pkh6",
			"signatureValue": "4v3cTB5u0/nA/xxrGU3gQ38IaP1MJJ7tQyPQtBtZmVLE36M96d2XRo0ArFyxQV2CsDMtP57n/vnvZWN88Du+Bg==",
			"type": "Ed25519Signature2018"
		}
	]
}
```

## Submit a Claim

Submit a claim with some custom data:

```go
// create a temperature claim with this new sensor entity as both the issuer and the subject of the claim
temperatureClaim, err := claim.New(
	claim.Data{"temperature": 100},
	claim.Issuer(temperatureSensor),
	claim.Subject(temperatureSensor),
)
if err != nil {
	log.Fatal(err)
}

// submit the claim to be
err = ockamChain.Submit(temperatureClaim)
if err != nil {
	log.Fatal(err)
}

fmt.Printf("Submitted - " + temperatureClaim.ID())
```

```
{
	"@context": [
		"https://w3id.org/identity/v1",
		"https://w3id.org/security/v1"
	],
	"id": "did:ockam:2PdDcphFfkW5eU1C1mFB1i9H8ZsgC/claim/iu5aczbwnt",
	"type": [
		""
	],
	"issuer": "did:ockam:2PdDcphFfkW5eU1C1mFB1i9H8ZsgC",
	"issued": "2019-01-10",
	"claim": {
		"id": "did:ockam:2PdDcphFfkW5eU1C1mFB1i9H8ZsgC",
		"temperature": 100
	},
	"signatures": [
		{
			"created": "2019-01-10T08:00:31Z",
			"creator": "did:ockam:2PdDcphFfkW5eU1C1mFB1i9H8ZsgC#key-1",
			"domain": "ockam",
			"nonce": "iu5aczbwnt",
			"signatureValue": "UpCPc/Z6bGwUXfgNgRFxpQU2kSt8HBoe8E94JyvlAKG1yBNBfqb4oUKdPZPHOQH37JtiIFap9eGS4qMBP35DDA==",
			"type": "Ed25519Signature2018"
		}
	]
}
```

## Build

The build and run ockam code:

```
./build && ./build install && ockam --version
```

This requires recent versions of Bash and Docker installed on your development machine.

You may also work within a Vagrant and Virtualbox environment, see details on that and other build tools in the
[Contributing Guide](CONTRIBUTING.md#contribute-code)

## Contributing to Ockam

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
* [Logan Jager](https://github.com/jagtek)
* [Matthew Gregory](https://github.com/mattgreg)
* [Mrinal Wadhwa](https://github.com/mrinalwadhwa)
* [Rolf Kaiser](https://github.com/rkaiser0324)
