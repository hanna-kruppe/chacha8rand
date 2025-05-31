# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - (release date TBD)

### Added

Improved and expanded `target_feature` detection for SIMD backends:
* The AVX2 backend can be used without `std` when the target feature is
  statically known to be available (most likely due to `-Ctarget-feature`).
* The SSE2 backend supports runtime feature detection with the `std` feature.
  This only matters for `i586-unknown-linux-{gnu,musl}` targets.
* AArch64 NEON backend also supports runtime feature detection with the `std`
  feature. This is for consistency with x86, it shouldn't matter for any
  existing target.

### Changed

* The minimum support Rust version is now 1.87
* SIMD backends use far less `unsafe` code than before

## [0.1.1] - 2025-03-23

### Added

* Support for rand v0.9 (opt-in feature `rand_core_0_9`)

### Removed

* Crate feature `unstable_internals` which was for internal use only and is no
  longer needed. Its existence and functionality were explicitly not covered by
  SemVer.

## 0.1.0 - 2024-10-14

Initial release.

[Unreleased]: https://github.com/hanna-kruppe/chacha8rand/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/hanna-kruppe/chacha8rand/compare/v0.1.0...v0.1.1
