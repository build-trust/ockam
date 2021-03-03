# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.4.0 - 2021-03-03
### Added

- Auto-implemenation of the `Message` trait for certain types.

### Modified

- The `Worker` trait and its methods are now async.
- Updated dependencies.

## v0.3.0 - 2021-02-16
### Added

- Explicit `alloc`, `no_std`, and `std` features.
- Generalized `Address` implementation.
- Global crate lib facade wrapper around `std` and `core` re-exports, for cross-feature compatibility.
- Message trait base implementation.

### Modified

- Updated dependencies.
- Improved documentation.



## v0.2.0 - 2021-02-04
### Added

-  Runs Worker `initialize` function when the Worker is started.
-  Uses `From` trait in place of `Into` in Node error
-  Create Worker example

### Removed
-  Worker Builder

### Modified

-  Moved Worker and Address types to this crate.
-  Renamed executor commands to messages

## v0.1.0 - 2021-01-30
### Added

- `Error` - an error type that can be returned is both `std` and `no_std` modes.
- `Result` - a result type that can be returned is both `std` and `no_std` modes.

