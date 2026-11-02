# Changelog

All notable changes to this project. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- Updated rea-launcher-sdk to v0.2.0 (crates, vendored `sdk/`, CI
  workflows, which now also run the Windows-target tests under Wine).
  Control borders in the settings window are lighter (`#6b6e75`) so they
  reach 3:1 contrast; the window fits the screen's DPI on native Windows
  as before (now opted in explicitly). `cmr4-play`'s tests load it with
  the SDK's `REA_FUNCTIONS_ONLY`/`REA_PLAY_HERE` convention; the old
  `CMR4_PLAY_FUNCTIONS_ONLY` hook is gone.
- Built on the shared [rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk)
  v0.1.0: the runtime copy, component ledger, `launcher.cfg` editing, patch
  and registry helpers, game start, settings-window kit and `cmr4-play`'s
  session handling (`rea-play.sh`, shipped beside it in the zip and the
  AppImage) are the SDK's. Files, settings, commands and environment names
  are unchanged.
- `launcher.cfg`: setting a key that is only present as a commented
  `#key =` line fills that line in place instead of appending one; `;`
  starts a comment too; a UTF-8 BOM is dropped; `×` is accepted in sizes.
- Settings window: the Browse button's text is white, the focused control
  shows a focus ring, and the d-pad's left/right step through a row of
  choices.
- `cmr4-play`: the settings window's errors go to
  `~/.config/cmr4-launcher/settings.log`; an interrupted launch also ends
  the prefix's wineserver; a `launcher.cfg` next to `CMR4Launcher.exe` is
  read first; `--settings`, `--status`, `--stop` and `--help` are accepted.
- Old `.cmr4-new` temp files in the runtime are removed once.

### Fixed

- An unknown `cmr4.exe` no longer stops the launch. Builds are identified
  by SHA-256 (`src/edition.rs`); the widescreen fix is a patch table
  applied only to the known 1.0.0.0 build (also when it already carries
  the community fix) and verified against its expected hash. Any other
  build runs unpatched, with the game's own view, and the window and log
  say so in plain words with the code to report and a link to the new
  "Unknown edition" issue form.
- Settings window under Wine: Wine's own DPI setting (e.g. 144 or 192 in
  `winecfg`) is divided out of the zoom the window is created with, so it is
  no longer enlarged twice (2.25x/3x, taller than a 4K screen).
- Saving a `launcher.cfg` written by v0.1.0/v0.2.0 replaces its header line
  "Command-line options override these values and are saved.", which is no
  longer true, with the current rule.
- `cmr4-play`: an interrupted launch (Ctrl+C, the AppImage closed) now waits
  for the game session to end after signalling it, so a start right after
  never finds the old session still running.
- Wine DLL overrides: a player's rule spelled with `.dll` (`d3d9.dll=b`, any
  case) for a DLL the launcher manages is replaced like `d3d9=b` instead of
  being kept next to the launcher's own.
- Game and runtime folders on drives that can't be canonicalized (Windows
  RAM disks, `subst` drives, some FUSE/network filesystems: "Incorrect
  function") are normalised lexically instead of refusing to launch; a
  missing game folder is still an error.
- `cmr4-play stop` also runs `wineserver -k` for the launcher's private
  prefix after ending the session group, so Wine processes outside the group
  (or left by a crashed launcher) end too.
- First run: the settings window opens on the recommended look (Steam Deck on
  a 1280x800 screen, 4K elsewhere) only when there is no `launcher.cfg` yet.
  Before, a Deck got the 4K look, and an unreadable file counted as a first
  run.
- `launcher.cfg` is no longer rewritten wholesale: saving from the window
  changes only the launcher's own keys and keeps your comments and other
  lines (and CRLF line endings). Unknown keys are logged as warnings.
- Command-line options are one-run overrides again: only the game folder is
  remembered in `launcher.cfg` (when it has none, or `--game` names another).
  Before, every flag (and the resolved runtime) was saved. A resolution the
  launcher picks from the screen, when neither the file nor `--resolution`
  sets one, is still saved, except inside gamescope: there the screen is
  `cmr4-play`'s 1024x768 fallback, which would otherwise be kept for good.
- A `launcher.cfg` that exists but can't be read stops the launch with an
  error instead of being replaced by defaults. One saved by Notepad in ANSI
  (not UTF-8), e.g. with an accented folder name, is read as Latin-1 instead
  of blocking every launch, and saved back as UTF-8.
- The game's `OPTIONS` save, the patched `cmr4.exe`, `dgVoodoo.conf`,
  `Xidi.ini` and the launcher's own bookkeeping files are written to a temp
  file and renamed into place, so a crash or full disk can't leave them torn.
  On Windows a rename refused while an antivirus scans the new file is
  retried a few times (5 × 100 ms).
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
