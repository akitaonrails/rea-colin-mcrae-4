# Colin McRae Rally 04 Launcher

Run your own installed PC copy of **Colin McRae Rally 04** on Windows 10/11,
or on Linux through Wine, Steam/Proton, or the AppImage. Point the launcher at
the game folder, pick a look, and press **Play**: the widescreen picture, the
graphics wrappers, the gamepad and the game's first-run state are set up for
you.

![CMR04 in a stage at 16:9, driven with a gamepad](docs/images/game.png)

This repository and its packages contain **no game files**. The launcher never
modifies the installed game folder; everything it changes lives in a private
copy (the *runtime*).

## What the launcher fixes

| Problem on a modern PC | What the launcher does |
| --- | --- |
| The game is 4:3, and its widescreen patch is easy to get wrong: rendered natively at 16:9 the 3D view comes out 25% too narrow (cars look squeezed). | Applies the tested "16:9 Low" patch to the private `cmr4.exe` only, lets the game render 4:3 (2880x2160 for a 4K screen) and has dgVoodoo stretch that frame to 16:9. That is the arrangement the patch was made for, so cars and scenery keep their real proportions with the wider widescreen view. |
| With a portrait monitor set as primary, the game and its wrapper fit the picture to the portrait screen; the compositor then stretches it into a thin, squashed strip on the landscape screen. | Picks the largest landscape monitor and hands it to dgVoodoo as its fullscreen output. Under Wine the game runs borderless; gamescope and native Windows use normal fullscreen. |
| The game ignores window-close requests: Alt+F4 and a compositor's close (Super+W in Hyprland) do nothing, so it had to be killed. | Loads a small hook (`cmr4close.dll`, built from this repository) into the game; Alt+F4 or closing the window ends it. Other windows' close messages (such as the intro movie's) are ignored. |
| Modern pads either don't drive at all or need a fiddly in-game setup; the game's own menu for choosing the pad can freeze under Wine. | Xidi presents the pad as a classic gamepad: stick steers, RT/LT are analogue pedals. For a profile whose controls were never set up, the launcher selects that pad, analogue pedals and their axis tuning in the game's `OPTIONS`, so a new player can drive straight away. Profiles set up by hand are left alone. |
| D3D9-era rendering and Wine don't mix well. | Installs the tested dgVoodoo2 2.80 `D3D9.dll`; under Wine/Proton it adds DXVK 3.1.1 so the game draws through Vulkan. |
| A fresh install needs registry paths and first-run defaults, or it crashes. | Writes the install paths every launch and seeds the game's graphics defaults only when they are missing. |
| Your installation and saves are at risk from patches and wrappers. | The installed folder is only read. The runtime keeps the patched executable, wrappers, settings and saves, and later launches never overwrite your saves, settings or your own DLL mods. |

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

- **Windows:** [CMR4Launcher-v0.1.0.zip](https://github.com/akitaonrails/rea-colin-mcrae-4/releases/latest/download/CMR4Launcher-v0.1.0.zip)
- **Linux AppImage:** [CMR4Launcher-v0.1.0-x86_64.AppImage](https://github.com/akitaonrails/rea-colin-mcrae-4/releases/latest/download/CMR4Launcher-v0.1.0-x86_64.AppImage)

## What you need

An installed Windows copy of **Colin McRae Rally 04** with `cmr4.exe` in its
folder. No installer, disc, executable, media or game data is supplied here.

The launcher accepts an executable with exactly one tested widescreen
signature, original or already patched. Anything else stops safely and shows
the executable's SHA-256; it never guesses another patch location. Put that
digest, not the executable, in a bug report.

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

**Play** holds the game folder, the look and the runtime folder. A look sets
the picture in one click:

| Look | Resolution | Widescreen | Frame-rate cap |
| --- | --- | --- | --- |
| **Original** | the game's own mode | off | unchanged |
| **4K** (recommended) | the largest exact 16:9 mode that fits your screen, up to 3840x2160 | on | unchanged |
| **Steam Deck** (recommended on a 1280x800 screen) | 1280x800, filling the 16:10 screen | off | 60 |
| **Custom** | your own choice in **Picture** | — | — |

The look itself isn't stored; the window works it out from the saved values.

![Picture tab](docs/images/launcher-picture.png)

**Picture** offers Original, a screen-fit mode when your screen isn't a
standard size, 1080p, 1440p and 4K; a frame-rate cap (uncapped, 30, 60, 120,
144); forced VSync; and the widescreen patch. Widescreen needs an exact 16:9
resolution; other shapes (4:3, 16:10, ultrawide) need widescreen off and can be
set in `launcher.cfg` or on the command line.

**Controls** turns the Xidi gamepad on or off and shows what the game sees.

Settings are saved when you press **Play**. The window works with a pad too:
D-pad moves, A activates, LB/RB switch tabs, Start plays.

## Gamepad

With **Controls → Gamepad through Xidi** on (the default):

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

With no arguments the settings window opens. Any option skips the window,
launches directly, and saves its values to `launcher.cfg` for next time.

```text
CMR4Launcher [options]
  --game <folder>       folder containing the installed cmr4.exe
  --runtime <folder>    private writable runtime folder
  --resolution WxH      exact 16:9 for widescreen, or original (default: screen fit)
  --widescreen on|off   tested 16:9 Low patch (default on; off for other aspects)
  --fps-limit N|off     dgVoodoo frame-rate cap
  --vsync on|off        force vertical sync (default off)
  --controller on|off   Xidi gamepad: stick steers, RT/LT are the pedals (default on)
  --check               validate the game and patch and save settings; copy nothing
  --no-window           run without the settings window
  --help                show this help
```

`launcher.cfg` uses the same names, one `key = value` per line (`#` starts a
comment):

| Key | Values |
| --- | --- |
| `game`, `runtime` | folders; `runtime` empty means the default |
| `resolution` | `WxH` (320–7680 × 240–4320) or `original` |
| `widescreen`, `vsync`, `controller` | `on`/`off` (also `true`/`false`, `yes`/`no`, `1`/`0`) |
| `fps_limit` | 1–1000, or `off` |

An invalid value is logged as a warning and the rest of the file still
applies.

## Files and logs

| What | Windows | AppImage / Wine |
| --- | --- | --- |
| `launcher.cfg` | `%LOCALAPPDATA%\CMR4Launcher\launcher.cfg` | in the prefix, under `AppData/Local/CMR4Launcher` |
| `launcher.log` | `%LOCALAPPDATA%\CMR4Launcher\launcher.log` | same place |
| Default runtime | `%LOCALAPPDATA%\CMR4Launcher\CMR4` | same place |

A `launcher.cfg` next to `CMR4Launcher.exe` takes precedence (portable setup).
The log records what the launcher did, including the monitor and dgVoodoo
output it chose and the `gamepad setup:` step. Remove personal paths before
posting logs.

## Troubleshooting

- **Unsupported executable:** send the SHA-256 from the error and your game
  edition. Do not attach `cmr4.exe`.
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
target.

```sh
cargo test --workspace
cargo clippy --release --target i686-pc-windows-gnu --all-targets -- -D warnings
cargo build --release --target i686-pc-windows-gnu
packaging/build-windows.sh dev .work/packages
sh linux/tests/gpu-env.sh
sh linux/tests/gamescope.sh
sh linux/tests/supervision.sh
sh packaging/tests/package.sh
```

GNU ld from mingw-w64 binutils 2.47 (snapshot builds in rolling distros) can
drop an import descriptor from the launcher, which then crashes at startup
under Wine; link with LLVM's lld in that case:
`RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo build --release --target i686-pc-windows-gnu`.
CI's Ubuntu toolchain is not affected.

Components are downloaded from official pinned URLs and checked by SHA-256
(`packaging/components.pins`); archives are never committed. The one exception
is `packaging/mirror/dgVoodoo2_80.zip`, holding only the tested 2.80
`D3D9.dll` (see its README). The package smoke test accepts only an exact file
list and rejects game assets.

## Legal

Colin McRae Rally 04 is © Codemasters. This project is not affiliated with or
endorsed by Codemasters or Electronic Arts. Our code is MIT OR Apache-2.0;
third-party binaries, fonts, the portrait photo and Wine, with their notices,
are listed in [THIRD_PARTY.md](THIRD_PARTY.md).
