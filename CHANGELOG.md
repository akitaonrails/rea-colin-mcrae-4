# Changelog

All notable changes to this project. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

- The game's `OPTIONS` save, the patched `cmr4.exe`, `dgVoodoo.conf`,
  `Xidi.ini` and the launcher's own bookkeeping files are written to a temp
  file and renamed into place, so a crash or full disk can't leave them torn.
- Wine: winebus `Map Controllers` is written only when the prefix has no
  value, so a player's own setting is kept instead of being reset to 1 on
  every launch.
- The Controls tab no longer tells players to choose the controller and
  analogue pedals in the game's Options; the launcher does that for a
  profile whose controls were never set up.
- AppImage build: the runtime is pinned to the dated type2-runtime release
  `20251108` instead of the `continuous` tag, which is rebuilt in place and
  would break the checksum pin.
- `THIRD_PARTY.md` lists Xidi under its real licence, BSD-3-Clause (the
  upstream `LICENSE` was already shipped in `licenses/Xidi/`).

### Changed

- The settings window looks XInputGetState up once instead of loading the
  XInput DLL on every frame.
- A golden fixture (`tests/fixtures/xidi.ini`) pins the generated
  `Xidi.ini` byte for byte.
- CI builds with `--locked`, pins its actions by commit, fails when a
  package artifact is missing, checks the tag against the crate version and
  publishes `SHA256SUMS` with each release.
- `.gitignore` also ignores `/dist/`, `/runtime-data/`, `/original-data/`
  and `/build/`, so local packages and game data can't be committed.

## [0.2.0] - 2026-11-02

### Added

- `cmr4close.dll` (a workspace crate): ends the game on Alt+F4 or a window
  close aimed at its window, which CMR04 itself ignores.
- Look presets (Original, 4K, Steam Deck, Custom) and a CMR04 palette in
  the settings window.
- For a profile whose controls were never set up, the launcher selects the
  connected pad, analogue pedals and axis tuning in the game's OPTIONS save.
  Hand-configured profiles are left alone.

### Fixed

- Widescreen: the game renders 4:3 and dgVoodoo stretches it to 16:9, which
  is what the 16:9 Low patch expects; native 16:9 had squeezed the 3D view.
- With a portrait primary display the picture goes to the largest landscape
  monitor (dgVoodoo `FullScreenOutput`) instead of a thin strip.
- Gamepad axes reach the game: Xidi keeps the StandardGamepad layout with
  RT/LT on the pedal axis instead of mapping the pad to keyboard keys.
- Turning Xidi (or DXVK on Windows) off no longer fails when the game folder
  ships its own DLL of that name.
- `cmr4-play` reads `launcher.cfg` resolutions as leniently as the launcher.
- Wine detection also accepts the Wine/Proton environment variables.

## [0.1.0] - 2026-11-01

The first release: a launcher that plays your own copy of Colin McRae
Rally 04 on Windows 10/11 and on Linux (Wine, Proton, AppImage).

### Added

- A private runtime copy of the game, synced from the installed folder,
  which is never changed; saves, settings and user DLLs survive later syncs.
- Registry install paths written on every launch, first-run graphics
  defaults added only when missing.
- The tested 16:9 Low widescreen byte patch, applied to the runtime copy by
  signature only (unknown executables fail safely and report their SHA-256).
- dgVoodoo2 2.80 D3D9, Xidi 4.3.1 and (under Wine/Proton) DXVK 3.1.1,
  installed with a SHA-256 ownership ledger that never deletes a user's DLL.
- Settings window, commented `launcher.cfg` and command-line flags.
- `linux/cmr4-play` (session lock, gamescope, GPU selection, `stop`) and a
  Linux AppImage with portable Wine 11.20.
- Windows zip and AppImage built in CI from pinned component archives, with
  a package inventory check that rejects unlisted files and game assets.

[Unreleased]: https://github.com/akitaonrails/rea-colin-mcrae-4/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/akitaonrails/rea-colin-mcrae-4/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/akitaonrails/rea-colin-mcrae-4/releases/tag/v0.1.0
