# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

## [1.2.0] - 2025-12-31
- [gdm/16](https://github.com/k0psutin/gdm/issues/16) Added support for git based addons/plugins.
  - See [README: Add command usage](https://github.com/k0psutin/gdm/blob/main/README.md#add)
- Update dependencies:
  - [pr/34](https://github.com/k0psutin/gdm/pull/34) Bump reqwest from 0.12.24 to 0.12.26
  - [pr/33](https://github.com/k0psutin/gdm/pull/33) Bump tracing from 0.1.41 to 0.1.43
  - [pr/31](https://github.com/k0psutin/gdm/pull/31) Bump http from 1.3.1 to 1.4.0
  - [pr/30](https://github.com/k0psutin/gdm/pull/30) Bump mockall from 0.13.1 to 0.14.0

## [1.1.0] - 2025-11-18
- [gdm/17](https://github.com/k0psutin/gdm/issues/17) **BREAKING CHANGES**
  - Changed internal plugin resolution logic. You must run `gdm install` before adding new plugins to update `gdm.json` (see README migration notes).
- [gdm/17](https://github.com/k0psutin/gdm/issues/17) Improved handling of assets with multiple plugin folders: main plugin is selected, others are marked as sub_addons.
- Optimized `Cargo.toml` features to reduce binary size.


## [1.0.1] - 2025-11-09
- [gdm/13](https://github.com/k0psutin/gdm/issues/13) Fix a bug where gdm gave an error if asset /addons folder contained files

## [1.0.0] - 2025-10-29
- Initial release
