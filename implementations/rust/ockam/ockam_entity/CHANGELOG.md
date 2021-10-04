# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.22.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.21.0 - 2021-09-27
### Changed
- Use forked version of crates core2 and serde_bare.
- Ockam compiles under no_std + alloc.
- Dependencies updated

## v0.20.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.19.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.18.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.17.0 - 2021-09-03
### Added
- Lease Manager client and JSON protocol
### Changed
- Dependencies updated.

## v0.16.0 - 2021-08-30
### Added
- Created ockam_transport_core crate for generic transport code
### Changed
- Dependencies updated.

## v0.15.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.14.0 - 2021-08-16
### Added
- Implement BLS signature using BBS+.
- Introduce Signature Vault type.
### Changed
- Dependencies updated.

## v0.13.0 - 2021-08-09
### Changed
- Ignore error while stopping secure channel listener.
- Dependencies updated.

## v0.12.0 - 2021-08-03
### Added
- Added a simple Entity builder.
### Changed
- Refactored entity secure channel workers.
- Dependencies updated.

## v0.11.0 - 2021-07-29
### Changed
- Clarify trust policy names.
- Fix creation of secure channel with known entity.
- Dependencies updated.
- Rename trust policies to more descriptive names.

## v0.10.0 - 2021-07-26
### Added
- Add credential protocol and implementation.
- Add add_credential and get_credential to Holder trait.
- Add reveal_attributes and credential_type macros.
- Add get_secure_channel_participant_id function.
- Add convenient creation functions to trust_policy_worker.
### Changed
- Entity create function now takes an optional id.
- Rename Credential to BbsCredential to avoid naming collision.
- Dependencies updated.

## v0.9.0 - 2021-07-19
### Added
- `credential_attribute_values` macro
- `credential_type` macro
### Changed
- Dependencies updated.
- Re-enable trust policies for secure channels, post refactor.

## v0.8.0 - 2021-07-12
### Added
- New `from_external` function to `ProfileIdentifier`, for creating identifiers from serialized forms.

### Changed
- Dependencies updated.
- Secure channel creation no longer panics when used with an entity.
- Move signing key to Profile change events.
- Entity Worker `get_contact` response changed to correct type.

## v0.7.0 - 2021-07-06
### Added
- Credential APIs based on Entities.
- `check_origin` function for `LocalMessage`.
### Changed
- Dependencies updated.

## v0.6.0 - 2021-06-30
### Added
- Identity trait for defining Profile behavior.
### Changed
- Entity and Profile implementation restructured.
- Fix clippy warnings.

## v0.5.0 - 2021-06-21
### Added
- Added LocalMessage for locally routed messages.
- Added `UnknownChannelMsgOrigin` error.
### Changed
- Renamed SecureChannelListener callback to `completed_callback_address`
- Make the ProfileChannelListener `listener_address` required.
- TransportMessage constructor has been extended to use recent routing changes.
- Dependencies updated.

## v0.4.0 - 2021-06-14
### Changed
- Dependencies updated.

## v0.3.0 - 2021-05-30
### Added
- Entity abstraction.
- Trust policy abstraction and IdentityTrustPolicy implementation.

### Changed
- Dependency updates.
- Fix clippy issues.

## v0.2.0 - 2021-05-17
- Documentation and meta-information fixes.

## v0.1.0 - 2021-05-17

- Initial release.

