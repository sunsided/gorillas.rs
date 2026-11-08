# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-11-08

### Changed

- GORILLAS.BAS is now ported statement by statement onto an emulated VGA card and PC speaker
  (game-toolkit-dosvideo), replacing the wgpu/cpal remake. With the same seed it draws the
  same city, gorillas, sun and explosions as QBasic 1.1 in DOSBox, pixel for pixel, and paces
  its animations and music like it.
- Licensed as EUPL-1.2 AND GPL-2.0-or-later: the embedded VGA BIOS fonts come from DOSBox-X.

### Added

- A browser build, on GitHub Pages and on games.markusmayer.dev.
- CRT filters (Alt+C), starting on the soft glow.
- `gorillas capture`: scripted headless sessions with screenshots, frames and WAV output, and a
  DOSBox-X comparison against the original (`task compare`).

[Unreleased]: https://github.com/sunsided/gorillas.rs/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/sunsided/gorillas.rs/releases/tag/v0.2.0
