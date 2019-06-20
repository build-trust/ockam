		# Decentralized Identifiers
As we noted above, Identification and Authenticity are critical components in establishing data integrity. Decentralized Identifier (DID) is a new type of identifier (standard under development at W3C) that can prove its authenticity unlike a MAC address, IP address or UUID.

DIDs are fully under the control of the DID subject (entity) and this control can be cryptographically proven.

In Ockam people, places, services, and things are modeled as entities. Each `Entity` has at least one DID that
begins with `did:ockam:` and uses the `ockam` DID method. An example Ockam DID looks like this: `did:ockam:2QyqWz4xWB5o4Pr9G9fcZjXTE2ej5`.

A DID is also a URL that resolves to a DID document. DID documents allow an Entity to declare its current public keys, authentication methods, communication endpoints and other metadata.

```
{
	"id": "did:ockam:2QyqWz4xWB5o4Pr9G9fcZjXTE2ej5"
	"publicKey": [
		{
			"id": "did:ockam:2QyqWz4xWB5o4Pr9G9fcZjXTE2ej5#key-1",
			"publicKeyHex": "3c93f446990ecd3ce64bcf9a5f949423d2e348948ee3aeb1c78924490f6b50f9",
			"type": "Ed25519VerificationKey2018"
		}
	],
	"authentication": [
		{
			"publicKey": "did:ockam:2QyqWz4xWB5o4Pr9G9fcZjXTE2ej5#key-1",
			"type": "Ed25519SignatureAuthentication2018"
		}
	]
}
```

An entity may control more than one DID to maintain privacy. It may then chose to use pairwise unique DIDs for each relationship to ensure that interactions in different contexts are not co-relatable.
