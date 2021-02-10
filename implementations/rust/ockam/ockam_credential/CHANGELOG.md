# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.1 - 2021-02-10
## Added

- Only available in std mode
- `Claim` - information that has been signed in a credential
- `ClaimType` - the data encoding for a claim
- `CredentialOffer` - information from an issuer that will be in a credential
- `Credential` - a cryptographic signature with claims
- `BlindCredential` - a cryptographic blinded signature with claims
- `CredentialError::MismatchedAttributesAndClaims` - schema attributes and claims do not match
- `CredentialError::MismatchedAttributeClaimType` - attribute type and claim type do not match
- `CredentialError::InvalidClaim` - claim data cannot be mapped to a valid cryptographic value
- `Issuer` - methods for issuing credentials and blinded credentials

## 0.1.0 - 2021-02-05
## Added

- `Schema` - a credential's data layout.
- `Attribute` - a statement by the credential issuer about the credential subject.
- `AttributeType` - the data type of an attribute's value.
