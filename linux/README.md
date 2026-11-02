# Colin McRae Rally 04 on Linux

The launcher is a Windows program. On Linux it runs through Wine or Proton,
and you can play three ways: the AppImage, which brings its own Wine; Steam
with Proton, which also covers the Steam Deck; or your own Wine with
`cmr4-play` from the zip.

## The AppImage

```sh
chmod +x CMR4Launcher-*-x86_64.AppImage
./CMR4Launcher-*-x86_64.AppImage
```

It includes portable Wine 11.20 (Kron4ek's WoW64 build) and the same pinned
DXVK 3.1.1 x32 DLLs as the Windows zip, so you don't need to install Wine.
It runs `cmr4-play` with that Wine first on `PATH`, sets `CMR4_LAUNCHER` and
`CMR4_WAIT`, keeps any Wine DLL overrides you already set, and turns off
Wine's .NET and HTML install prompts, which the game doesn't need.

Its prefix is `~/.local/share/cmr4-launcher/prefix`, and gamescope is used
when it is installed. Stages with stick steering and analogue RT/LT pedals
were driven this way (through gamescope) with an 8BitDo pad.

## Steam (Proton)

1. Unzip the Windows package, e.g. into `~/Games/cmr4-launcher`.
2. **Games → Add a Non-Steam Game to My Library…** and pick
   `CMR4Launcher.exe`.
3. **Properties → Compatibility**: force GE-Proton or Proton Experimental.
4. Start it, browse to the game, press **Play**.

Proton brings its own DXVK and runs the launcher without `cmr4-play`. This
route has not been tested yet.

### Steam Deck

The same steps, from Desktop Mode. On the Deck's 1280x800 screen the
window recommends the **Steam Deck** look and can be driven with the
controller. Neither Desktop nor Game Mode has been tested on a Deck yet.

## Your own Wine

The zip contains `linux/cmr4-play`. It runs the Windows launcher through
Wine, then starts the game from the private copy:

```sh
linux/cmr4-play          # settings window through Wine, then the game
linux/cmr4-play --play   # saved settings, no window (alias: start)
linux/cmr4-play status   # is our session running?
linux/cmr4-play stop     # stop our session, then the prefix's wineserver
```

It needs Wine. If gamescope is installed and not already running, the
helper uses the configured resolution for both the game's size and the
output, fits it to the screen and scales with the fixed `nis` filter (there
is no scaler setting). Without gamescope it starts the game directly. To
run Wine and gamescope inside a container such as a distrobox, set
`CMR4_RUN` to a wrapper command.

The helper uses a Wine prefix of its own and waits for the game in every
mode. An unrelated
process named `cmr4.exe` neither blocks the helper nor gets its stop
signal. `stop` then runs `wineserver -k` for the launcher's own prefix
only, which also ends Wine processes left outside the session. The lock
records the process and its start time and is removed on exit. Paths with
spaces or apostrophes are passed as arguments, never pasted into shell
code.

## Environment

| Variable | Meaning | Default |
| --- | --- | --- |
| `CMR4_PREFIX` | Wine prefix | `~/.local/share/cmr4-launcher/prefix` |
| `CMR4_LAUNCHER` | `CMR4Launcher.exe` path | beside the helper or one folder up |
| `CMR4_RUN` | A wrapper the helper runs Wine and gamescope through (a distrobox, for example) | none |
| `CMR4_WAIT` | Accepted for the AppImage; every launch already waits | — |
| `DRI_PRIME`, `__NV_PRIME_RENDER_OFFLOAD` | Your GPU choice is kept | set automatically only when unset |
| `SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS` | Your value is kept | `1` |
| `SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS` | Your value is kept, separately | `1` |

## Where things are

| What | Where |
| --- | --- |
| Settings (`launcher.cfg`) and log (`launcher.log`) | in the prefix, `drive_c/users/<you>/AppData/Local/CMR4Launcher/` |
| The launcher's copy of the game, with your saves | in the prefix, `drive_c/users/<you>/AppData/Local/CMR4Launcher/CMR4` |
| Wine prefix | `~/.local/share/cmr4-launcher/prefix` |
| `cmr4-play`'s session lock and the settings window's error output (`settings.log`) | `~/.config/cmr4-launcher` |

Under Proton the launcher's files are in Steam's `compatdata/<appid>/pfx`
instead.

## Troubleshooting

- **`--play` starts in a small 1024x768 window every time:** gamescope's
  size comes from `resolution` in the prefix's `launcher.cfg`. When that is
  `original`, missing or invalid, the helper uses 1024x768, and inside
  gamescope the launcher doesn't save the size it sees. Open the settings
  window once (`linux/cmr4-play`) and press **Play**.
- **The settings window doesn't open:** its error output is in
  `~/.config/cmr4-launcher/settings.log`.
- **"already running":** `linux/cmr4-play status` shows whether our session
  is still running; `linux/cmr4-play stop` ends it.

`cmr4-play` holds only CMR04's choices. The session, GPU, gamescope and
prefix handling is the shared play library `rea-play.sh` from
[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk), shipped
beside it in the zip and the AppImage (`sdk/rea-play.sh` in this
repository). Its tests (`linux/tests/*.sh`) use fake commands and harmless
helper processes only.
