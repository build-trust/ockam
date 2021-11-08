# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.3.0 - 2021-11-08
### Added
- add hygiene module
- add "no_main" feature to "node" macro
### Changed
- Dependencies updated
- node macro infers `Context` and `Result` types

## v0.2.0 - 2021-11-01
### Changed
- Dependencies updated

## v0.1.0 - 2021-10-16

Initial release.

### Added
- `node` - a proc macro that defines a custom `#[node]` attribute.
- `node_test` - a proc macro that defines a custom `#[node_test]` attribute.
