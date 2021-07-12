# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

