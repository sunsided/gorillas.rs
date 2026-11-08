# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Licensed as EUPL-1.2 only again: the VGA fonts are game-toolkit-dosvideo 0.2.2's built-in
  ones, from Joseph Gil's public-domain fntcol16 collection, instead of DOSBox-X's
  (GPL-2.0-or-later). The 8x14 font is the same; the 8x16 text-mode font differs from DOSBox's in
  19 glyphs (`0 M T V-Z m v-x` and a few others).
- game-toolkit-dosvideo 0.2.2: the overscan border is shown again, now centered (8 dots per side
  in `SCREEN 9`).

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
