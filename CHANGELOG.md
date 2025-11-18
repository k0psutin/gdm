# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

## [1.1.0] - 2025-11-18
- [gdm/17](https://github.com/k0psutin/gdm/issues/17) **BREAKING CHANGES**
  - Changed internal plugin resolution logic. You must run `gdm install` before adding new plugins to update `gdm.json` (see README migration notes).
- [gdm/17](https://github.com/k0psutin/gdm/issues/17) Improved handling of assets with multiple plugin folders: main plugin is selected, others are marked as sub_addons.
- Optimized `Cargo.toml` features to reduce binary size.


## [1.0.1] - 2025-11-09
- [gdm/13](https://github.com/k0psutin/gdm/issues/13) Fix a bug where gdm gave an error if asset /addons folder contained files

## [1.0.0] - 2025-10-29
- Initial release
