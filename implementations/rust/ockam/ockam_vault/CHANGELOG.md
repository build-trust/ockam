# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased
### Added
### Changed
### Removed

## v0.30.0 - 2021-10-25
### Changed
- Fix zeroize usage.
- Make vault async.
- Simplified feature usage.
- Move as many things as possible into a workspace.
- Dependencies updated
### Remove
- Remove `None` errors from Error enums.

## v0.29.0 - 2021-10-18
### Changed
- Make credentials optional (disabled by default)
- Dependencies updated

## v0.28.0 - 2021-10-11
### Changed
- Dependencies updated

## v0.27.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.25.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.24.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.23.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.22.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.21.0 - 2021-08-30
### Changed
- Dependencies updated.

## v0.20.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.19.0 - 2021-08-16
### Added
- Add key validation during secret_import in ockam_vault.
- Implement BLS signature using BBS+.
- Introduce Signature vault type.
### Changed
- Dependencies updated.
- Remove clamping from curve25519 key generation.

## v0.18.0 - 2021-08-09
### Changed
- Dependencies updated.

## v0.17.0 - 2021-08-03
### Changed
- Dependencies updated.

## v0.16.0 - 2021-07-29
### Added
- Add key id computation while importing secret in ockam_vault.
### Changed
- Dependencies updated.

## v0.15.0 - 2021-07-26
### Changed
- Dependencies updated.

## v0.14.0 - 2021-07-19
### Changed
- Dependencies updated.

## v0.13.0 - 2021-07-12
### Added
- Placeholder for BLS signature Vault implementation.
### Changed
- Dependencies updated.

## v0.12.0 - 2021-07-06
### Added
- Type for `BLS` secrets.

### Changed
- Dependencies updated.

## v0.11.0 - 2021-06-30
### Added
- Identity trait for defining Profile behavior.
### Changed
- Entity and Profile implementation restructured.
- Fix clippy warnings.
- Dependencies updated.

## v0.10.0 - 2021-06-21
### Changed
- Dependencies updated.

## v0.9.0 - 2021-06-14
### Changed
- Dependencies updated.

## v0.8.0 - 2021-05-30
### Added
### Changed
- Dependencies updated.

## v0.7.0 - 2021-05-17
### Added
### Changed
- Dependencies updated.
- Use result_message in Vault Worker.
- Refactors in support of Entity abstraction.

## v0.6.0 - 2021-05-10
### Added
### Changed
- Dependencies updated.
- Documentation edits.
### Deleted

## v0.5.2 - 2021-05-03
### Changed
- Dependencies updated.

## v0.5.1 - 2021-04-26
### Changed
- Dependencies updated.

## v0.5.0 - 2021-04-33
### Changed
- Crate dependency reorganization.

## v0.4.1 - 2021-04-19
### Changed
- Dependencies updated.

## v0.4.0 - 2021-04-14
### Changed
- Improved asymmetric trait.
- Improved hasher trait.
- Improved key_id trait.
- Improved verify trait.
- Build system and test fixes.

## v0.3.4 - 2021-04-13
### Changed
- Dependencies updated.

## v0.3.3 - 2021-04-12
### Changed
- Dependencies updated.

## v0.3.2 - 2021-04-06
### Changed
- Dependencies updated.

## v0.3.1 - 2021-03-23
### Changed
- Dependencies updated.

## v0.3.0 - 2021-03-03
### Changed

- Traits renamed for consistency.
- Documentation edits.

## v0.2.0 - 2021-02-16
### Added
- Symmetric and Asymmetric SoftwareVault implementations.

### Changed
- Dependencies updated.
- Fixes to error propagation and entity construction.

## v0.1.0 - 2021-02-10

Initial release.

