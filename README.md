# Colin McRae Rally 04 Launcher

Run your own installed PC copy of **Colin McRae Rally 04** on Windows 10/11,
or on Linux through Wine, Steam/Proton, or the AppImage. Point the launcher at
the game folder, pick a look, and press **Play**: the widescreen picture, the
graphics wrappers, the gamepad and the game's first-run state are set up for
you.

[![A USA stage in the Lancer Evo with v0.2.0: the launcher's look presets, then 16:9 widescreen on the landscape monitor, driven with a gamepad's stick and analogue triggers](https://new-uploads-akitaonrails.s3.us-east-2.amazonaws.com/2026/11/02/colin-mcrae-rally-04/showcase-poster.jpg)](https://new-uploads-akitaonrails.s3.us-east-2.amazonaws.com/2026/11/02/colin-mcrae-rally-04/showcase-h264.mp4)

*Version 0.2.0 from the settings window to a full USA stage: correct 16:9
proportions on the landscape monitor, driven with a pad's stick and analogue
triggers. Click to watch (2⅓ minutes, with sound).*

This repository and its packages contain **no game files**. The launcher never
modifies the installed game folder; everything it changes lives in a private
copy (the *runtime*).

## Problems it solves

Each item below is a problem the game has on a modern PC, followed by what
the launcher does about it.

**Runs on today's computers**

- **Crashes on a fresh install.** CMR04 needs install paths and first-run
  graphics defaults in the registry. The launcher writes the paths on every
  launch and adds the defaults only when they are missing.
- **Old Direct3D 9 renderer.** The launcher installs the tested dgVoodoo2
  2.80 wrapper. Under Wine or Proton it also adds DXVK, so the game draws
  through Vulkan on your graphics card.
- **Closing the game does nothing.** CMR04 ignores Alt+F4 and window-close
  requests (Super+W in Hyprland), so it had to be killed. A small hook
  built from this repository (`cmr4close.dll`) now ends it when you press
  Alt+F4 or close its window.

**Picture**

- **Widescreen that squeezes the cars.** The known 16:9 patch is easy to
  get wrong. Rendered directly at 16:9, cars and scenery come out 25% too
  narrow. The patch was made for a 4:3 picture stretched to 16:9, so the
  launcher has the game render 4:3 (2880x2160 for a 4K screen) and dgVoodoo
  stretch it. You get correct proportions and the wider view.
- **A thin, squashed strip with two monitors.** With a portrait monitor
  set as primary, the picture was fitted to that portrait screen and then
  stretched onto the landscape one. The launcher now sends the game to
  your largest landscape monitor.
- **Picking settings.** Choose a look in one click: Original, Widescreen
  (the largest 16:9 size that fits your screen), Steam Deck or Custom.
  The widescreen view follows the resolution: any 16:9 size gets it,
  other shapes keep the game's own view. There is also an optional
  frame-rate cap and VSync.
- **Still to do:** the HUD (speedometer, timer) is drawn for 4:3 and stays
  slightly wide at 16:9. Fixing it would need another patch to the game.

**Gamepads**

- **The pad works in menus but not in a race.** In our tests with Wine
  alone, the pad's buttons reached the game but its stick and triggers
  didn't, and the old mapping turned the pad into keyboard keys, which also
  hid the axes. Bundled Xidi now presents a classic gamepad: the
  left stick steers, RT and LT are analogue pedals, and a resting thumb on
  the stick can't open the throttle.
- **Every new player had to set up the pad.** New profiles start on the
  keyboard, and choosing the controller in the game's own menu can freeze
  it under Wine. For a profile whose controls were never set up, the
  launcher selects your pad, analogue pedals and the game's axis tuning,
  so you can drive straight away. A profile you set up yourself is left as
  it is.

**Your files**

- **An unknown version of the game.** The launcher knows its `cmr4.exe`
  builds by their SHA-256 and patches only those. Any other version still
  plays, with the game's own view, and the window offers a button to
  report it so it can be added.
- **Patches and wrappers could damage your install.** The installed game
  folder is only read, never changed. A private copy (the runtime) holds
  the patched `cmr4.exe`, the wrappers, settings and saves.
- **Updates could overwrite your saves.** Later launches update the
  runtime without overwriting your saves, settings or DLL mods you added
  yourself.

![CMR04 in a stage at 16:9, driven with a gamepad](docs/images/game.jpg)

## Verification status

Verified on Linux (Wine 11.21, RTX 5090, 4K screens, an 8BitDo Ultimate 2):

- **Picture:** correct 16:9 proportions in stages with widescreen on (4:3
  render stretched to 3840x2160), filling the landscape monitor with a portrait
  primary monitor present.
- **Closing:** Super+W ends the game.
- **Gamepad:** with Xidi on, stick steering and analogue RT/LT pedals in a
  stage; menus with the D-pad and A. A brand-new profile set up by the launcher
  drove without opening Options.
- **Settings window:** presets, palette and gamepad navigation render under
  Wine.

Not yet verified: native Windows, Steam Deck/Proton separately, protected
retail discs, and other pad models. Details and the evidence behind each fix:
[docs/RUNTIME_NOTES.md](docs/RUNTIME_NOTES.md) and
[docs/RESEARCH.md](docs/RESEARCH.md).

## Download

From the [latest release](https://github.com/akitaonrails/rea-colin-mcrae-4/releases/latest):

- **Windows:** `CMR4Launcher-<version>.zip`
- **Linux AppImage:** `CMR4Launcher-<version>-x86_64.AppImage`

## What you need

An installed Windows copy of **Colin McRae Rally 04** with `cmr4.exe` in its
folder. No installer, disc, executable, media or game data is supplied here.

The launcher knows `cmr4.exe` 1.0.0.0 (also with the community 16:9 fix
already applied) by its SHA-256 and applies the widescreen fix only to it.
Any other version plays unpatched, with the game's own view; the window
and `launcher.log` show its SHA-256 and a link to the
[Unknown edition](https://github.com/akitaonrails/rea-colin-mcrae-4/issues/new?template=unknown-edition.yml)
form. Report that code, never the executable.

Original protected retail discs may not work under Wine (older reports say
so; DRM-free/budget editions were reported working). This project has not
verified either.

## Install and run

### Windows

1. Download the zip from
   [Releases](https://github.com/akitaonrails/rea-colin-mcrae-4/releases) and
   unzip it anywhere outside the game folder.
2. Keep `CMR4Launcher.exe` and the `components/` folder together and run the
   executable.
3. On **Play**, browse to the folder with `cmr4.exe`, pick a look, and press
   **Play**.

The first launch copies the game into the runtime (a few GB); later launches
only update what changed.

### Linux: AppImage (no Wine install needed)

```sh
chmod +x CMR4Launcher-*-x86_64.AppImage
./CMR4Launcher-*-x86_64.AppImage
```

It bundles portable Wine 11.20 and uses the prefix
`~/.local/share/cmr4-launcher/prefix`. Your Linux files appear as drive `Z:`
when browsing for the game. Gamescope is used when installed. See
[linux/README.md](linux/README.md).

### Linux: Steam / Proton

1. Unzip the Windows package, e.g. into `~/Games/cmr4-launcher`.
2. **Games → Add a Non-Steam Game to My Library…** and pick
   `CMR4Launcher.exe`.
3. **Properties → Compatibility**: force GE-Proton or Proton Experimental.
4. Start it, browse to the game, press Play.

Steam Deck works the same way from Desktop Mode; Game Mode is not verified.

### Linux: your own Wine (and gamescope)

The zip contains `linux/cmr4-play`:

```sh
linux/cmr4-play          # settings window, then the game
linux/cmr4-play --play   # saved settings, no window (alias: start)
linux/cmr4-play status   # is our session running?
linux/cmr4-play stop     # end our session
```

It needs Wine; gamescope is optional.

## The settings window

![Play tab with the look presets](docs/images/launcher.png)

**Play** holds the game folder, the look and the private copy folder. A
look sets the picture in one click:

| Look | Resolution | View | Frame-rate cap |
| --- | --- | --- | --- |
| **Original** | the game's own mode | the game's own | unchanged |
| **Widescreen** (recommended) | the largest 16:9 size that fits your screen, up to 3840x2160 | widescreen | unchanged |
| **Steam Deck** (recommended on a 1280x800 screen) | 1280x800, filling the 16:10 screen | the game's own | 60 |
| **Custom** | your own choice in **Picture** | — | — |

The look itself isn't stored; the window works it out from the saved values.
The first time it opens (no `launcher.cfg` yet) it starts on the recommended
look for your screen.

![Picture tab](docs/images/launcher-picture.png)

**Picture** offers Original, your screen's own 16:9 fit when it isn't a
standard size, 1920x1080, 2560x1440 and 3840x2160 (16:9), 1600x1200 (4:3),
1920x1200 (16:10) and 2560x1080 (ultrawide); a frame-rate cap (no cap, 30,
60, 120, 144); and VSync. A 16:9 size gets the widescreen view (the game
draws a 4:3 picture that dgVoodoo stretches, which keeps the cars' shape);
other shapes keep the game's own view. A value set in `launcher.cfg` or on
the command line that isn't in the list shows as "(saved)" and is kept.
(The screenshots predate the Widescreen look's new name.)

**Controls** turns the Gamepad on or off and shows what the game sees.

Settings are saved when you press **Play**. The window works with a pad
(D-pad moves, A activates, LB/RB switch tabs, Start plays) and with the
keyboard alone (Tab moves, arrows pick, Ctrl+Tab switches tabs, Enter plays).

## Gamepad

With **Controls → Gamepad** on (the default; it uses the bundled Xidi):

| On the pad | In the game |
| --- | --- |
| Left stick | Steering (analogue) |
| RT / LT | Accelerate / brake (analogue) |
| D-pad, A, B | Menus |
| X, Y, LB, RB … | Handbrake, camera, gears (the game's defaults) |

On launch, a profile that still has the factory controls (keyboard, digital
pedals, untouched bindings) gets the connected pad, analogue pedals and the
axis tuning the game itself writes, so you can drive without opening Options.
The pad is chosen by its USB IDs, so **wake a wireless pad before launching**;
if none is connected the profile is left as it is and set up next time.

If you set the controls up yourself (or change pads), do it in
**Options → Controllers**: pick the controller, set Accel/brake to Analogue,
then select Accelerate and pull RT, Brake and pull LT, Turn left/right and move
the stick. The game labels the trigger axis "Z axis"; that is expected.
Choosing the controller in that menu can freeze the game under Wine; the choice
is saved before the freeze, so restart and it is in effect.

With Xidi off the game reads the pad through Windows/Wine directly. Buttons and
the D-pad work, but under Wine the pad's axes did not drive the car in our
tests, so keep Xidi on unless your setup needs otherwise.

## Command line and `launcher.cfg`

With no arguments the settings window opens. Any option skips the window and
launches directly. Options apply to that run only; the one remembered in
`launcher.cfg` is the game folder (`--game`). When neither the file nor
`--resolution`/`--look` sets a resolution, the one the launcher picks from
the screen is saved too, except inside gamescope, where the screen is only
gamescope's own size.

```text
CMR4Launcher [options]

  --game <folder>                        Game folder
  --look original|widescreen|steam_deck  Look
  --resolution <WxH>                     Resolution
  --fps-limit <0-1000>                   Frame-rate cap
  --vsync on|off                         VSync
  --controller on|off                    Gamepad
  --runtime <folder>                     Private copy folder
  --check                                check the game and settings only
  --no-window                            play with saved settings and flags
  --settings                             only the settings window (Linux helper)
  --linux                                started by the Linux helper (cmr4-play)
  --help                                 show this help

With no options the settings window opens. Options override launcher.cfg
for one run; only --game is remembered there. The game folder is never
modified.
```

`launcher.cfg` uses the same names with underscores (`fps_limit` for
`--fps-limit`), one `key = value` per line (`#` starts a comment). The
first run writes every key with its description:

| Key | Values |
| --- | --- |
| `game`, `runtime` | folders; `runtime` empty means the default |
| `look` | `original`, `widescreen` or `steam_deck`: sets those values; values set next to it win |
| `resolution` | `WxH` (320–7680 × 240–4320) or `original`; 16:9 gets the widescreen view |
| `fps_limit` | 0–1000; `0` or `off` is no cap |
| `vsync`, `controller` | `on`/`off` (also `true`/`false`, `yes`/`no`, `1`/`0`) |

An invalid value or an unknown key is logged as a warning and the rest of the
file still applies. `widescreen` (removed in 0.3.0: the resolution decides)
is accepted and ignored, as a key and as `--widescreen`. Saving from the
window or remembering `--game` changes only the launcher's own lines: your
comments and any other lines are kept.

## Files, saves and uninstalling

| What | Windows | AppImage / Wine |
| --- | --- | --- |
| `launcher.cfg` | `%LOCALAPPDATA%\CMR4Launcher\launcher.cfg` | in the prefix, under `drive_c/users/<you>/AppData/Local/CMR4Launcher` |
| `launcher.log` | `%LOCALAPPDATA%\CMR4Launcher\launcher.log` | same place |
| Private copy (runtime) | `%LOCALAPPDATA%\CMR4Launcher\CMR4` (or `runtime`) | same place |
| Wine prefix | — | `~/.local/share/cmr4-launcher/prefix` |

A `launcher.cfg` next to `CMR4Launcher.exe` takes precedence (portable setup).

**Your saves live only in the private copy.** The game writes its profiles
and settings (`OPTIONS`), championships (`CHAMPIONSHIP_SLOT_*`) and best
times (`BESTTIMES_GHOSTS`) next to the runtime's `cmr4.exe`. The launcher
never overwrites them when it updates the copy, and it never writes them
back to the game folder.

**To uninstall:** first **back up your saves** (the files above, from the
runtime folder). Then delete the launcher's folder, and
`%LOCALAPPDATA%\CMR4Launcher` on Windows, or the prefix
`~/.local/share/cmr4-launcher` (and `~/.config/cmr4-launcher`) on Linux.
On Windows the launcher also wrote the game's registry key
(`HKLM\SOFTWARE\WOW6432Node\Codemasters\Colin McRae Rally 04`, or the
same under `HKCU\Software\Classes\VirtualStore`); remove it only if you
no longer play the game itself. The installed game folder was never
changed.

`launcher.log` records what the launcher did and why: the game version and
its SHA-256, the look and settings, each bundled DLL with its SHA-256, the
DLL overrides given to the game, the monitor and dgVoodoo output it chose
and the `gamepad setup:` step. It starts afresh past 1 MiB. Remove
personal paths before posting it.

## Troubleshooting

- **"This cmr4.exe is a version the launcher doesn't know yet":** the game
  still plays, without the widescreen view. Press **Report this version…**
  (or open the [Unknown edition](https://github.com/akitaonrails/rea-colin-mcrae-4/issues/new?template=unknown-edition.yml)
  form) and give the code it shows and where your copy comes from. Do not
  attach `cmr4.exe`.
- **The pad doesn't drive:** keep Xidi on; wake the pad before launching; check
  `gamepad setup:` in the log; set the controls once in Options → Controllers as
  described above.
- **The picture is in the wrong place or squashed:** the log's
  `display/render policy` line names the monitor and dgVoodoo output; report it
  with your monitor layout.
- **Runtime DLL conflict:** the launcher refuses to replace a DLL it didn't
  install. Move your custom DLL aside or use a fresh runtime.
- **In-game Advanced Graphics menu:** avoid it; it rewrote the resolution and
  hung in testing. The launcher reapplies its resolution on every launch.

## For developers

The workspace builds the launcher and the close hook (`close-hook/`, producing
`cmr4close.dll`). The toolchain is Rust plus `mingw-w64` for the 32-bit Windows
target, Wine for the Windows-target tests, and `cargo-deny`; the
[`gaming` distrobox](https://github.com/akitaonrails/distrobox-gaming) has
them all.

```sh
cargo fmt --all --check
cargo test --workspace                     # host: logic, plans, GUI (headless)
cargo clippy --release --target i686-pc-windows-gnu --all-targets -- -D warnings
cargo build --release --locked --target i686-pc-windows-gnu
sdk/scripts/wine-test --locked --workspace # Windows-target tests under Wine, headless
cargo deny check
sh ../rea-launcher-sdk/scripts/sdk-sync --check .
for t in linux/tests/*.sh packaging/tests/*.sh; do sh "$t"; done
packaging/build-windows.sh dev .work/packages
```

Tests never open a window and never need the game. The settings window is
tested with `egui_kittest` (`src/e2e.rs`, `src/window.rs`); every option is
tested from window control and flag to what it writes, through
`plan::plan`. `CMR4_GAME_DIR=<game folder> cargo test -- --ignored` also
checks the widescreen patch against your real `cmr4.exe` (read only).

GNU ld from mingw-w64 binutils 2.47 (snapshot builds in rolling distros) can
drop an import descriptor from the launcher, which then crashes at startup
under Wine; link with LLVM's lld in that case:
`RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo build --release --target i686-pc-windows-gnu`.
CI's Ubuntu toolchain is not affected.

Components are downloaded at package time from pinned URLs and checked by
SHA-256 (`packaging/components.pins`); archives are never committed. The
licence-less dgVoodoo2 2.80 archive comes from this repository's
`thirdparty` release (see `packaging/mirror/README.md`). The package smoke
test accepts only an exact file list and rejects game assets.

## Built on rea-launcher-sdk

The runtime copy, settings window kit, `launcher.cfg` option table, patch
tables, Win32 helpers, the Linux play library (`rea-play.sh`), packaging
scripts and CI workflows come from
[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk)
(v0.2.0), shared by the rea launchers. This repository keeps what is
specific to Colin McRae Rally 04.

## Contributing

- **Something about CMR04** (its versions, picture, gamepad, saves, the
  settings, `cmr4-play`'s game settings): issues and pull requests here.
- **Shared code** (the runtime folder logic, settings-window widgets,
  `rea-play.sh`, packaging, the CI workflows, anything under `sdk/`):
  issues and pull requests at
  [rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk).
- **How to tell:** if the file is under `sdk/`, or the code comes from a
  `rea_launcher*` crate, it is the SDK's. Never patch `sdk/` here and never
  copy SDK code into this repository; a fix lands in the SDK and reaches
  this launcher with an SDK update.
- Never attach game files (executables, saves, data) to an issue.

## Credits

- **AuToMaNiAk005**: the 16:9 "Low" widescreen fix the launcher applies.
- **Dege**: [dgVoodoo2](https://dege.freeweb.hu/dgVoodoo2/dgVoodoo2/).
- **Philip Rebohle and contributors**: [DXVK](https://github.com/doitsujin/dxvk).
- **Samuel Grossman**: [Xidi](https://github.com/samuelgr/Xidi).
- **Chris Robinson and contributors**: [DSOAL](https://github.com/kcat/dsoal),
  which the launcher keeps when you installed it.
- **Carolyn Williams**: the photo of Colin McRae in the window
  ([CC BY 2.0](https://www.flickr.com/photos/8365241@N02/1429137255/)).
- **Jeremy Tribby**: the [Barlow](https://github.com/jpt/barlow) fonts (SIL OFL 1.1).
- **WineHQ** and **Kron4ek**: Wine and the portable build in the AppImage;
  **AppImage** project: appimagetool and its runtime.
- [PCGamingWiki](https://www.pcgamingwiki.com/wiki/Colin_McRae_Rally_04)'s
  CMR04 page, where the fixes above were found.

## Legal

Colin McRae Rally 04 is © Codemasters. This project is not affiliated with or
endorsed by Codemasters or Electronic Arts. Our code is MIT OR Apache-2.0;
third-party binaries, fonts, the portrait photo and Wine, with their notices,
are listed in [THIRD_PARTY.md](THIRD_PARTY.md).
