# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).


## [Unreleased]
[Unreleased]: https://github.com/althonos/pyskani/compare/v0.2.0...HEAD


## [v0.2.0] - 2025-08-21
[v0.2.0]: https://github.com/althonos/pyskani/compare/v0.1.3...v0.2.0

### Added
- Support for reading and writing `Database` in "consolidated" format (introduced in `skani v0.3.0`).
- `compression` and `marker_compression` properties to allow accessing the sketch parameters of a `Database` object.

### Changed
- Bump `pyo3` dependency to `v0.25.1`.
- Bump `skani` dependency to `v0.3.0`.


## [v0.1.3] - 2024-12-05
[v0.1.3]: https://github.com/althonos/pyskani/compare/v0.1.2...v0.1.3

### Changed
- Bump `pyo3` dependency to `v0.22.5`.
- Bump `skani` dependency to `v0.1.2`.
- Use `maturin` instead of `setuptools-rust` to compile package.
- Use PyData theme to render the Sphinx documentation to HTML.

### Fixed
- Missing documentation for some keyword arguments of `Database` methods.


## [v0.1.2] - 2023-04-11
[v0.1.2]: https://github.com/althonos/pyskani/compare/v0.1.1...v0.1.2

### Changed
- Bumped `pyo3` to `v0.21.0`.

### Added
- Wheels for CPython 3.12 and PyPy 3.10.


## [v0.1.1] - 2023-04-11
[v0.1.1]: https://github.com/althonos/pyskani/compare/v0.1.0...v0.1.1

### Changed
- Bumped `skani` to `v0.1.1`.
- Use read-write locks for synchronizing database contents.

### Fixed
- Generation of AUR package in GitHub Actions workflow.

### Docs
- Display the wrapped `skani` version in the Sphinx documentation.


## [v0.1.0] - 2023-02-09
[v0.1.0]: https://github.com/althonos/pyskani/compare/a851bd...v0.1.0

Initial release.
