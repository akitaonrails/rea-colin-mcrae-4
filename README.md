# Colin McRae Rally 04 Launcher

Run your own installed PC copy of **Colin McRae Rally 04** on Windows 10/11,
or on Linux through Wine, Steam/Proton, or the AppImage. Point the launcher at
the game folder and press **Play**: the modern display, renderer, and controller
setup is done for you.

![CMR04 running in 16:9 at 3840x2160](docs/images/game.png)

## What the launcher solves

- **Modern picture:** an exact 16:9 mode that fits your screen, up to 4K, with
  the tested widescreen correction and no stretched 4:3 image.
- **Difficult displays and GPUs:** borderless fullscreen under raw Wine avoids
  portrait/wrong-monitor selection; gamescope and native Windows use normal
  fullscreen. Hybrid graphics use the discrete GPU where the platform allows it.
- **Old graphics dependencies:** dgVoodoo2 and, only under Wine/Proton, DXVK
  are installed and configured for you. No separate wrapper downloads or
  compatibility-mode instructions.
- **Gamepad setup:** Xidi maps a modern Xbox-style controller to steering,
  pedals, menu buttons, and D-pad navigation out of the box.
- **Fragile first-run state:** the launcher creates the required registry paths
  and display defaults, then reapplies its owned resolution each launch.
- **Your installation and saves:** the installed folder is read-only. A private
  runtime holds the widescreen patch, wrappers, configuration, and saves.

The launcher never modifies the installed game folder. It makes a persistent
private runtime containing the patched executable, compatibility wrappers,
configuration, saves, and game settings. This repository and its packages
contain **no game files**.

> **Verification status:** the exact final Windows package and AppImage are
> verified under Linux/Wine. A fresh prefix reached the 3840x2160 title and main
> menu with DXVK; the 3D attract path was also reached earlier. The AppImage
> navigated title, options, gearbox, and car-class screens using a virtual Xbox
> pad. Source executable/settings hashes stayed unchanged. Native Windows, a
> full stage with analog steering/pedals, separate Steam Deck/Proton testing,
> raw-desktop visual capture, and protected retail media remain unverified. See
> [docs/RUNTIME_NOTES.md](docs/RUNTIME_NOTES.md).

## Download

- **Windows:** [CMR4Launcher-v0.1.0.zip](https://github.com/akitaonrails/rea-colin-mcrae-4/releases/latest/download/CMR4Launcher-v0.1.0.zip)
- **Linux AppImage:** [CMR4Launcher-v0.1.0-x86_64.AppImage](https://github.com/akitaonrails/rea-colin-mcrae-4/releases/latest/download/CMR4Launcher-v0.1.0-x86_64.AppImage)

## What you need

An installed Windows copy of **Colin McRae Rally 04**, with `cmr4.exe` in its
folder. Install the game before running this launcher; no installer, disc,
executable, media, or game data is supplied here.

The launcher accepts an executable with exactly one tested widescreen
signature, whether original or already patched. Missing or ambiguous signatures
stop safely and display the executable's SHA-256. It never searches for a
similar patch location. Include that digest—not the executable—in a test report.

Original protected retail media may not work under Wine: older PlayOnLinux
notes report problems, while a DRM-free/budget edition was reported working.
This project has not independently verified those reports or disc protection
compatibility.

## Windows: unzip and play

1. Download `CMR4Launcher-….zip` from
   [Releases](https://github.com/akitaonrails/rea-colin-mcrae-4/releases) and
   unzip it anywhere outside the installed game folder.
2. Keep `CMR4Launcher.exe` and `components/` together, then double-click the
   executable.
3. On **Play**, browse to the installed folder containing `cmr4.exe`. The
   window validates the PE32 executable and widescreen signature.
4. Leave the runtime path empty for the default, or choose another private
   folder outside the game. Press **Play**.

The first launch copies the game into the private runtime. Later launches
update ordinary game files without replacing persistent saves, settings, or
runtime DLL mods. The source installation is only read.

## Linux

### AppImage — no Wine installation needed

Download `CMR4Launcher-…-x86_64.AppImage`, then:

```sh
chmod +x CMR4Launcher-*-x86_64.AppImage
./CMR4Launcher-*-x86_64.AppImage
```

The AppImage includes the launcher, components, and portable Wine 11.20. It
uses `~/.local/share/cmr4-launcher/prefix`. Browse to the installed game (your
Linux filesystem appears as drive `Z:`), then press Play. Gamescope is used
when available; otherwise the game runs directly. See
[linux/README.md](linux/README.md).

### Steam / Proton

1. Unzip the Windows package, for example into `~/Games/cmr4-launcher`.
2. In Steam choose **Games → Add a Non-Steam Game to My Library…** and select
   `CMR4Launcher.exe`.
3. Open **Properties → Compatibility**, force a Steam Play compatibility tool,
   and choose GE-Proton or Proton Experimental.
4. Start it, browse to the installed game, and press Play.

Steam Deck follows the same Desktop Mode steps. Deck/Game Mode behavior is
pending runtime verification; reports are welcome through the test-report
issue template.

### Existing Wine and gamescope

The zip also contains `linux/cmr4-play`:

```sh
linux/cmr4-play          # settings, then game
linux/cmr4-play --play   # saved settings, no window
linux/cmr4-play status
linux/cmr4-play stop
```

It requires Wine; gamescope is optional. No container or distrobox is assumed.

## Settings and defaults

![The CMR04 settings window](docs/images/launcher.png)

Only these CMR04 settings are exposed:

| Setting | Default | Meaning |
| --- | --- | --- |
| Game folder | required | Installed folder containing `cmr4.exe`; read-only |
| Runtime folder | launcher default | Persistent private writable copy |
| Resolution | largest exact 16:9 fitting the active screen, up to 3840x2160 | Forced dgVoodoo mode and CMR04 Width/Height, or Original; 1920x1080 if screen unknown |
| Widescreen | on | Tested 16:9 Low byte patch on the private executable only |
| FPS limit | off | Optional dgVoodoo frame-rate cap |
| VSync | off | Optional forced vertical sync |
| Controller | on | Fixed Xidi modern-controller mapping |

Widescreen requires an exact 16:9 custom resolution. Original, 4:3, 16:10,
and ultrawide modes require widescreen off rather than stretching the Low patch.
Under Wine/Proton the package installs DXVK 3.1.1 (`d3d11.dll` and `dxgi.dll`);
native Windows does not install or require those files. Raw Wine uses
borderless fullscreen to avoid wrong-monitor exclusive enumeration; gamescope
and native Windows use normal fullscreen with aspect-preserving scaling.

Original resolution turns widescreen off. The gamepad mapping is fixed: left
stick/D-pad emit arrows, RT/LT accelerate/brake, A/Start confirm, B/Back
cancel, X is Space, and Y is C. There are no custom button, scaler, texture,
draw-distance, language, saves-folder, rumble, or HD-art options.

> **Avoid the in-game Advanced Graphics menu after using the modern setup.**
> The tested game rewrote its resolution there and could hang. The launcher
> reapplies its configured Width and Height on every launch, but changing the
> in-game menu is still not recommended.

## Troubleshooting

- **Unsupported executable:** read the SHA-256 in the error and submit it with
  your game edition. Do not attach `cmr4.exe`.
- **Missing components:** keep the supplied `components/` folder beside the
  launcher. `--check` validates only the game and patch without installing it.
- **Controller conflict:** the launcher refuses to overwrite unrecognized
  runtime-only DLLs. Move your custom DLL aside deliberately or use a fresh
  runtime; the source installation remains untouched.
- **Black screen or graphics-menu problem:** return to the launcher defaults
  and avoid Advanced Graphics. Runtime behavior is still being verified.
- **Linux display/input issue:** try the AppImage or `cmr4-play`, which can use
  gamescope and does not restrict input to a particular pad model.

Settings and log locations:

| What | Windows | AppImage / Wine |
| --- | --- | --- |
| `launcher.cfg` | `%LOCALAPPDATA%\CMR4Launcher\launcher.cfg` | Inside the prefix's `AppData/Local/CMR4Launcher` |
| `launcher.log` | `%LOCALAPPDATA%\CMR4Launcher\launcher.log` | Same prefix location |
| Default runtime | `%LOCALAPPDATA%\CMR4Launcher\CMR4` | Same prefix location |

A `launcher.cfg` beside `CMR4Launcher.exe` takes precedence for portable
setups. Logs record launcher activity, never the full process environment.
Remove personal paths before posting logs or settings.

## For developers

```sh
cargo test
sh linux/tests/gpu-env.sh
sh linux/tests/gamescope.sh
sh linux/tests/supervision.sh
sh packaging/tests/package.sh
cargo clippy --all-targets -- -D warnings
cargo build --release --target i686-pc-windows-gnu
packaging/build-windows.sh dev .work/packages
```

Component archives are downloaded from official pinned URLs into a cache and
verified by SHA-256 before exact archive members are installed; those archives
are never committed. The sole exception is the intentional, documented
`packaging/mirror/dgVoodoo2_80.zip` containing only the tested 2.80 D3D9 DLL.
`packaging/components.pins` records the download pins. The package
smoke test requires an exact allowlisted inventory and rejects unexpected game
assets or other files.

GitHub Actions runs host/shell tests, Windows lint/build, package assembly and
smoke checks, uploads artifacts on pushes and pull requests, and publishes
releases for `v*` tags. Research and verification status:
[docs/RESEARCH.md](docs/RESEARCH.md) and
[docs/RUNTIME_NOTES.md](docs/RUNTIME_NOTES.md).

## Legal

Colin McRae Rally 04 is © Codemasters. This project is not affiliated with or
endorsed by Codemasters or Electronic Arts. Our code is MIT OR Apache-2.0;
third-party compatibility binaries, fonts, portrait, Wine, and their notices
are listed in [THIRD_PARTY.md](THIRD_PARTY.md).
