# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Changed
- **Renamed project** from `rWifi` to `rWifi-tui`. The GitHub repository, Cargo package name, binary name, and all user-facing labels now use the `-tui` suffix to make the program's role as a terminal user interface explicit (matching `rTemplate-tui`).
  - Repository: `local76/rWifi` → `local76/rWifi-tui`
  - Crate/binary: `rwifi` → `rwifi-tui`
  - Console title: `rWifi` → `rWifi-tui`
  - Config file: `%APPDATA%\rWifi\config.yaml` → `%APPDATA%\rWifi-tui\config.yaml`
  - Log file: `%APPDATA%\rWifi\log.txt` → `%APPDATA%\rWifi-tui\log.txt`
  - Linux package names: `rwifi` → `rwifi-tui`

## [3.0.1] - 2026-06-06
### Added
- Added author and maintainer metadata for packaging.

## [3.0.0] - 2026-06-06
### Changed
- Renamed organization to `local76`.
- Renamed executable from `rtem` to `rwifi`.
- Reorganized directory structure to group packaging files inside `dist/packages/`.