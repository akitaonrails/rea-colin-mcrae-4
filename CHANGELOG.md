# Changelog

All notable changes to this project. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Removed

- **The Widescreen switch** (`widescreen`, `--widescreen`). It only ever
  worked together with an exact 16:9 resolution, so the resolution now
  decides: a 16:9 size gets the widescreen view, other shapes the game's
  own view (owner decision, 2026-11-02). An old `widescreen = …` line or
  `--widescreen` flag is accepted and ignored; saving from the window
  comments the line out. A player who had `widescreen = off` with a 16:9
  resolution now gets the widescreen view; pick a 4:3 or 16:10 size (or
  Original) to keep the game's own view.

### Added

- One launcher at a time. Starting a second `CMR4Launcher.exe` brings the
  open settings window to the front, or, while the game is running, says
  so and doesn't start a second copy.
- `--look original|widescreen|steam_deck` (and `look =` in
  `launcher.cfg`): sets a look's values for one run, like the window's
  look cards. Values set explicitly next to it win.
- Picture tab: 4:3 (1600×1200), 16:10 (1920×1200) and ultrawide
  (2560×1080) resolutions; a resolution or frame-rate cap saved from the
  file or the command line that isn't listed shows as a "(saved)" choice
  and is kept.

### Changed

- A `cmr4.exe` the launcher doesn't know (anything but 1.0.0.0, plain or
  with the community 16:9 fix) no longer gets the widescreen fix: the
  launcher only patches builds it has identified by SHA-256 (rea launcher
  standard: no patching by guesswork). The game still runs with its own
  view, and the message now says so and asks for the code so we can add
  that build.
- Updated rea-launcher-sdk to v0.3.2 (crates, vendored `sdk/` with the
  SDK's standards and playbooks in `sdk/docs/`, CI workflows). The Linux
  helper now reads `display_mode`, the resolution and the scaler in any
  case, and `--flag=value` works like `--flag value`. Screen
  readers now name the two folder fields by their headings ("Game
  folder", "Private copy folder").
- The option table now uses the SDK for what the launcher did by hand: the
  `off` spelling of `fps_limit`, the removed `widescreen` key, the
  switches in `--help`, Ctrl+Tab/Ctrl+PageDown tab switching, the chip
  rows' and Play's screen-reader text. `--help`, `launcher.cfg` and the
  window are unchanged.
- The close hook is now the SDK's `rea_close_hook.dll`
  (`rea-launcher-close-hook`, set to `class=PC_CMR4;strategy=terminate`)
  instead of our own `cmr4close.dll`. It behaves the same: Alt+F4 or
  closing the game's window ends the game, and the intro movies are left
  alone.
- README and `linux/README.md` follow the rea launcher layout: a
  problems table, screenshots, separate First run, Options (one table with
  every key, flag, window label and unset value), Where things live,
  Known limitations and Uninstall sections, and Steam Deck, Proton,
  Environment and Troubleshooting sections for Linux. Download links name
  no release; the screenshots are JPEG (`docs/images/launcher-play.jpg`,
  `launcher-picture.jpg`, `gameplay.jpg`).
- Issue forms follow the rea launcher layout: "Bug report" (it replaces
  "Test report" and keeps its "What worked?" checklist), "Unknown
  edition", and a link to the SDK for shared launcher code.
- CI: a weekly `sdk-update` workflow opens a pull request when a newer
  rea-launcher-sdk tag exists, listing what the update asks of the
  launcher. It never merges.
- Agent instructions: `CLAUDE.md` imports `AGENTS.md` and the SDK's
  vendored `sdk/docs/STANDARDS.md`; `AGENTS.md` keeps only CMR04's own
  rules and owner decisions and points to the standards and playbooks.
- Docs: README rewritten for this release (download links that don't
  name an old version, where saves live and how to uninstall, the
  generated `--help`, developer checks including `wine-test` and
  `cargo deny`, Built on rea-launcher-sdk, Contributing, Credits);
  THIRD_PARTY lists the SDK, appimagetool, the community widescreen fix,
  the README screenshots and video, and the verified photo source;
  `docs/RESEARCH.md` has the community-project table, the edition table
  and a survey of old-game trouble spots; `docs/CONTRIBUTIONS.md` holds
  the CMR04 drafts for PCGamingWiki; AGENTS.md points to the SDK's
  standards and playbooks. The game screenshot is a JPEG (408 KB).
- Packaging: `dgVoodoo2_80.zip` is no longer in Git; it is fetched from
  the repository's `thirdparty` release, pinned by SHA-256 (the owner
  uploads it once; see `packaging/mirror/README.md`). The package is
  unchanged.
- One option table (`config::OPTIONS`) declares every setting once: the
  `launcher.cfg` parser, the flags, `--help` (now listing `--settings` and
  `--linux` too) and the first-run template come from it, and a test
  checks the window has a control for each. The template's unset values
  are `#key =` lines now; files from earlier versions read as before.
- Settings window, in plain words: the "4K" look is now **Widescreen**
  (its note names the size it picks for your screen), every Picture row
  says what it does, the line under Resolution says what you'll see
  (widescreen view or the game's own view), the Gamepad switch replaces
  "Gamepad through Xidi", and the game-folder check says "Found cmr4.exe
  1.0.0.0" instead of "PE32 found; widescreen check: Patched". The photo
  caption sits on a solid band. The window's drawing moved into the
  library (`src/window.rs`), so it is tested headless.
- Settings window accessibility: Ctrl+Tab / Ctrl+Shift+Tab (and
  Ctrl+PageDown/PageUp) change tabs, so the keyboard reaches every tab;
  error messages say how to fix the problem; an unknown game version
  gets a "Report this version…" button that opens the issue form; the
  Controls tab no longer shows the pad mapping when Gamepad is off.
- `launcher.log` also records the look, each bundled DLL's SHA-256 and
  version, and starts afresh once it is past 1 MiB (the SDK's cap; it
  used to grow without end). `--help`, `--check` and errors print to the terminal the
  launcher was started from (or a redirected output); a dialog appears
  only when there is none.
- A launch is planned first (`src/plan.rs`: files, registry values, the
  game's environment, patch sets), then carried out; `launcher.log` now
  records the components installed and taken out and the game's
  `WINEDLLOVERRIDES`. A disabled Play button tells screen readers why.
- Golden snapshots (`tests/snapshots/`, `src/snapshots.rs`): `--help`, the
  first-run `launcher.cfg`, and the full launch plan for each look and
  three option combinations (native Windows, Wine on a second monitor
  without the gamepad, gamescope with an unknown build), so an SDK update
  can't change what the launcher writes unnoticed.
- Tests: every option end to end (window control and flag → the same
  `launcher.cfg` key → dgVoodoo.conf, Xidi, the registry, the environment
  or the patch), headless GUI tests of every tab (`egui_kittest`, on the
  host and under Wine), keyboard and pad navigation, an upgrade test with
  a v0.2.0 settings file, and game/runtime paths with apostrophes and
  accents.
- `fps_limit = 0` means no cap, like `off` (it already did; the docs said
  1–1000).

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

- Errors reach the player again under `cmr4-play`, the AppImage and
  Proton. Since the SDK v0.3.0 update an error such as a wrong game
  folder went only to the helper's log, because the launcher took the
  redirected output for a terminal, so the game just never started. Now
  any error that stops a launch shows a dialog whenever a person may be
  watching (under Wine always, on Windows unless `--no-window` was given)
  and is printed to the log too. `--help` and `--check` still print to the
  terminal, with a dialog only when nobody sees it (SDK
  `rea_launcher_win32::report`).
- A second start looked for the open launcher by its window title only, so
  an Explorer window on a folder named "Colin McRae Rally 04", or the
  game's own window, counted as the launcher: the second start exited
  without a word. It now accepts only a window of the launcher's own
  program (`CMR4Launcher.exe`), and otherwise says the game is running.
- `--check` while another launcher (or the game it started) is open now
  says so and exits with 1, instead of bringing that window forward and
  exiting 0, which a script read as a passed check.
- `cmr4-play`: a look set in `launcher.cfg` or with `--look`, with no
  resolution next to it, sized gamescope at the 1024x768 fallback: the
  Steam Deck look's 1280x800 ran in a 1024x768 output, and Widescreen
  shrank to 1024x576 (the largest 16:9 in that output). gamescope
  now gets the look's size (Steam Deck 1280x800, Widescreen the screen,
  Original 1024x768), and `--resolution`/`--look` on `cmr4-play --play`
  count for that run like they do for the launcher.
- The close hook's setup (`REA_CLOSE_HOOK=class=PC_CMR4;strategy=terminate`)
  is part of the launch plan: `launcher.log` lists it with the game's
  other environment values, and the golden snapshots show it. The hook is
  installed by the SDK's export name (`rea_close_hook::EXPORT`) instead of
  a copy of it. `RUNTIME_NOTES.md` names the screenshots by their real
  `.jpg` names.
- The Windows build is linked with LLD (`.cargo/config.toml`, i686 and
  x86_64). GNU ld 2.47 left some kernel32 imports of `CMR4Launcher.exe`
  outside every import descriptor, so the launcher could crash on its
  first call to one of them; `sdk/packaging/check-imports.sh` now finds
  none.
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
