# CMR04 on Linux

## `cmr4-play`

Runs the Windows launcher through Wine, then starts the private CMR04 runtime.
If gamescope is installed and not already active, the helper uses the configured
resolution for both nested and output size, fullscreen fit, and the fixed `nis`
filter. There is no scaler setting. Without gamescope it launches directly.

Gamescope's size comes from `resolution` in the prefix's `launcher.cfg`. When
that is `original`, missing or invalid it uses 1024x768, so a `--play` before
any settings were saved starts small; open the settings window once first.

```sh
linux/cmr4-play          # settings window through Wine, then game
linux/cmr4-play --play   # saved settings, no window (alias: start)
linux/cmr4-play status   # owned session only
linux/cmr4-play stop     # stop the owned session, then the prefix's wineserver
```

Wine is required for this standalone helper. The AppImage supplies it. The
helper uses a dedicated Wine prefix and synchronous supervision in every mode.
An unrelated process named `cmr4.exe` neither blocks nor receives its stop
signal. `stop` then runs `wineserver -k` for the launcher's own prefix only,
which also ends Wine processes left outside the session. Lock records include
process start identity and are removed on exit.

Optional environment:

| Variable | Meaning | Default |
| --- | --- | --- |
| `CMR4_PREFIX` | Wine prefix | `~/.local/share/cmr4-launcher/prefix` |
| `CMR4_LAUNCHER` | `CMR4Launcher.exe` path | beside the helper or one directory above |
| `CMR4_RUN` | Re-entrant wrapper for Wine/gamescope commands | none |
| `CMR4_WAIT` | Accepted for AppImage compatibility; every launch already waits | — |
| `DRI_PRIME`, `__NV_PRIME_RENDER_OFFLOAD` | Existing GPU choice is preserved | automatic only when unset |
| `SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS` | Existing value is preserved | `1` |
| `SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS` | Existing value is preserved independently | `1` |

Launcher paths containing spaces or apostrophes are passed as arguments, not
interpolated into shell source. Tests use fake commands and harmless helper
processes only:

```sh
sh linux/tests/gpu-env.sh
sh linux/tests/gamescope.sh
sh linux/tests/supervision.sh
```

## AppImage

The AppImage includes portable Wine 11.20 (Kron4ek WoW64 build) and the same
pinned DXVK 3.1.1 x32 DLLs as the Windows zip, so no Wine installation is
needed. `AppRun` puts that Wine first on `PATH`, sets
`CMR4_LAUNCHER` and `CMR4_WAIT`, preserves existing Wine overrides while
disabling unneeded .NET/HTML prompts, and executes `cmr4-play`.

Its default prefix is `~/.local/share/cmr4-launcher/prefix`. The launcher's
configuration and log are under the prefix user's
`AppData/Local/CMR4Launcher`. Stages with stick steering and analogue
RT/LT pedals were driven through `cmr4-play` (gamescope) with an 8BitDo pad;
separate Proton/Deck testing and protected retail media remain unverified.

Build from an allowlisted unpacked Windows package:

```sh
packaging/appimage/build.sh <unpacked-package-folder> <output-folder>
```

The build downloads Wine, appimagetool, and the AppImage runtime from their
official releases and verifies pinned SHA-256 digests. Wine source and LGPL notices are included in
the AppImage under `usr/share/doc/wine`.
