# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.11.0 - 2021-06-30
### Added
### Changed
- Fix clippy warnings.
- Dependencies updated.

## v0.10.0 - 2021-06-21
### Added
- Added LocalMessage for locally routed messages.
### Changed
- Standardize all Ockam crates to use the same version of `tokio`.
- Dependencies updated.

## v0.9.0 - 2021-06-14
### Changed
- Dependencies updated.

## v0.8.0 - 2021-05-30
### Added
### Changed
- Dependencies updated.
- Replace TCP Transport type with const.

## v0.7.0 - 2021-05-17
### Added
### Changed
- Dependencies updated.

## v0.6.3 - 2021-05-10
### Added
### Changed
- Documentation edits.
### Deleted

## v0.6.2 - 2021-05-03
### Changed
- Dependencies updated.

## v0.6.1 - 2021-04-26
### Changed
- Dependencies updated.

## v0.6.0 - 2021-04-22
### Changed
- Crate dependency reorganization.

## v0.5.1 - 2021-04-19
### Changed
- Dependencies updated.

## v0.5.0 - 2021-04-15
### Changed
- Improved TCP Transport API.

## v0.4.0 - 2021-04-14
### Added
- Added dead_code lint.
- Enabled multi-hop routes via domain specific routers.

### Changed
- Improved TCP transport initialisation.
- Improved the flow of the TCP Transport API.
- Dependencies updated.
- Build system and test fixes.


## v0.3.0 - 2021-04-13
### Changed
- Improved TCP echo example.
- Gracefully handle TCP connection failures.
- Improved printability of messages and payloads.
- Improved logging for dropped TCP connections.
- `msg_addr` moved from `Context` to `Routed`.
- Dependencies updated.
- Renamed Context address functions.
- Refactored Node Context API.
- Renamed `Routed` message wrapper function API.
- Simplified TCP Worker API for most common use cases.
- Take TCP addresses as strings and parse internally.

## v0.2.0 - 2021-03-22
### Added
- Route metadata wrapper type.
- New implementations of TCP Router and TCP Listener.

### Changed

- Dependencies updated.
- Split TCP worker into two parts: sender & receiver.


## v0.1.0 - 2021-02-10
### Added

- `Connection` - a trait that represents transport connections.
- `Listener` - a trait that represents transport connection listeners.
- `TcpConnection` - a TCP implementation of Connection.
- `TcpListener` - a TCP implementation of Listener.
