# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to [Semantic
Versioning].

## [Unreleased]

## [0.2.1] - 2021-04-15

### Fixed

- Bump to v0.2.1, to re-trigger a build in docs.rs. The previous build failed
  due to an issue in Rust nightly ([rust/rust-lang#84162]).

## [0.2.0] - 2021-04-13

### Changed

- Move the RocksDB support behind the `with-rocksdb` feature flag. This is done
  mainly to reduce the (re)build times.

## [0.1.0] - 2020-03-04

Initial release.

[Keep a Changelog]: https://keepachangelog.com/en/1.0.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html

[Unreleased]: https://github.com/apyrgio/caves/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/apyrgio/caves/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/apyrgio/caves/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/apyrgio/caves/releases/tag/v0.1.0

[rust/rust-lang#84162]: https://github.com/rust-lang/rust/issues/84162
