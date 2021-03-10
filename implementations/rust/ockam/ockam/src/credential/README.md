# Credential Description

This crate contains all the methods and data types needed to issue, prove, and verify the exchanging of credentials. This crate uses [BBS+](https://crypto.stanford.edu/~xb/crypto04a/groupsigs.pdf) signatures, which enables multiple attributes to be signed into a single signature and presented with zero-knowledge proofs. Other short group signatures like [PS](https://eprint.iacr.org/2015/525.pdf) or [Groth](https://acmccs.github.io/papers/p683-camenischA.pdf) are also acceptable.

## Structs

1. Credential - all the attributes that were signed and a short group signature
1. CredntialIssuer - represents an issuer of a credential
1. CredentialHolder - represents a holder of a credential
1. CredentialVerifier - represents all methods for verifying presentations
1. CredentialAttribute - enumeration of the possible signed values in the signaure
1. CredentialAttributeSchema - indicates the attribute types in a credential schema
1. CredentialAttributeType - indicates the attribute data type in a credential schema
1. CredentialSchema - describes the data format of a credential
1. CredentialOffer - a credential offer is how an issuer informs a potential holder that a credential is available to them
1. CredentialRequest - a request for a credential generated from a credential offer
1. CredentialFragment1 - output from creating a credential request by the holder that will be used with CredentialFragment2 to produce a Credential
1. CredentialFragment2 - output from processing a credential request from the holder by the issuer used with CredentialFragment1 to produce a Credential
1. CredentialPresentation - indicates a presented credential
1. PresentationManifest - list of the accepted schemas, public keys, and required to be revealed attributes from a verifier

## Methods

CredentialIssuer::create\_offer - creates a CredentialOffer to send to a holder.
CredentialIssuer::create\_proof\_of\_possession - creates a proof of possession over an issuer's credential signing key
CredentialIssuer::sign\_credential - produces a Credential for a holder where the issuer knows all attributes
CredentialIssuer::sign\_credential\_request - produces a CredentialFragment2 where the issuer only knows a subset of attributes to be signed. One or more attributes are contained in CredentialFragment1 known only to the holder.

CredentialHolder::accept\_credential\_offer - converts a CredentialOffer from the issuer into a CredentialRequest and CredentialFragment1. CredentialFragment1 contains one or more attributes known only to the holder. CredentialRequest is sent to the issuer.
CredentialHolder::combine\_credential\_fragments - combines CredentialFragment1 and CredentialFragment2 to produce a Credential
CredentialHolder::is\_valid\_credential - verify that a Credential is cryptographically verifiable to the issuer's public key.
CredentialHolder::present\_credentials - convert a credential and list of attributes into a zero-knowledge proof

CredentialVerifier::create\_proof\_request\_id - creates a proof request id as part of three pass presentation protocol.
CredentialVerifier::verify\_proof\_of\_possession - checks a proof of possession over an issuer's signing key.
CredentialVerifier::verify\_credential\_presentations - checks if a credential presentation is valid
