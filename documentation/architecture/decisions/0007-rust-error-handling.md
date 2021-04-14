# 7. Rust Error Handling

Date: 2021-04-06

## Status

Proposed

## Context

Error handling is a very important process that is needed by every crate of the original Ockam rust library, as well as any third-party crate that was designed to supplement Ockam rust library (such as transport and vault implementations).

There are multiple requirements to error handling:
  - agile enough to be used in different cases
  - portable to work in different environments with different constraints
  - convenient for both library developer and library user

## Decision

In search of balance between above-mentioned requirements it has been decided that errors are handled in native to Rust way of using Result type, Error type for such Result should be either of ockam_core::Error of implement Into<ockam_core::Error>

## ockam_core::Error

ockam_core::Error has following declaration:

```rust
pub struct Error {
    code: u32,

    #[cfg(feature = "std")]
    domain: &'static str,
}
```

There are following rules for creating errors:
  - DOMAIN_NAME is a static &str that is chosen for specific crate and must be unique, therefore its value should be tied to crate's name to avoid duplicates.
  - For no_std environments there is no &str error domain, therefore there is also DOMAIN_CODE, that has same uniqueness constraint and is added to every error number.
  - Error code is a number that is sum of DOMAIN_CODE and a unique error number specific to given error cause for this crate. Usually Error number is designed as an enum with first member being None (0).

## Consequences

  - There is some coupling because of DOMAIN_CODE uniqueness requirement, until there is deterministic way of calculating this code, developer must ensure newly created DOMAIN_CODE is unique.
  - Because domain is static &str, it's impossible to deserialize it, which puts some constraints on error usage. The other option would be to change type to String.
