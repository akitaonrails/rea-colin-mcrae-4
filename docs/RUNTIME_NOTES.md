# Runtime verification notes

This page distinguishes controlled runtime evidence from coverage that is
still unknown. Portable tests and package inspection support the implementation
but are not treated as gameplay evidence.

## 2026-11-03: rea-launcher-sdk v0.4.1 bump, headless real-game check

Launcher on branch `sdk-update/v0.4.1` (SDK v0.4.1), release build
packaged with `packaging/build-windows.sh` into `.work/bump-041-dist`;
same `cmr4.exe` (`7dca9f4c…1b41`), Wine 11.21 and gamescope 3.17.0 in
the `gaming` box. As on 2026-11-02 (`gamescope --backend headless`,
private prefix, `HOME`/`XDG_*` under `.work/rg041/`, PipeWire with one
null sink and WirePlumber's device monitors off, XTest and a small
Windows helper for input), with a new harness (`.work/rg041/bin/`): every
run starts from `env -i`; our processes are matched by our prefix or
runtime folder silently, listed by pid, state and name only, and only
they are signalled. The box has no screen, so a stand-in `xrandr`
reports a 1920x1080 desktop. The game folder's hashes were the same
afterwards and no file in it was newer than the first run.

| Check | Result |
|---|---|
| `--look widescreen` / `original` / `steam_deck` through `cmr4-play --play` (`REA_LOOK_SIZES`) | **VERIFIED**: gamescope 1920x1080 / 1024x768 / 1280x800, game resolution 1920x1080 / original / 1280x800, the game draws its intro (`look-*/shot.jpg`) |
| `look = steam_deck` in `launcher.cfg`, no flag | **VERIFIED**: gamescope 1280x800, the launcher logs the look from the file |
| Play-time error under `cmr4-play` (`--game` a missing folder) | **VERIFIED**: dialog "game folder Z:/mnt/no-such-cmr4: File not found." inside gamescope; OK ends the run |
| Second `CMR4Launcher.exe --no-window` while the game runs | **VERIFIED**: dialog "Colin McRae Rally 04 is already running. Quit the game (Alt+F4 works)…", **exit 5**, game untouched |
| Second `cmr4-play --play` while the game runs | **VERIFIED**: "already running", **exit 5** |
| Launcher killed while the game runs (plain Wine, no `cmr4-play`), then a second start | **VERIFIED**: the game kept running; the second launcher got past the instance check and `find_running` refused Play: same dialog, exit 5, game untouched. Under `cmr4-play` killing the launcher ends the whole session, so this case only arises without the helper |
| Second start while the settings window is open (private Xvfb, fresh prefix) | **VERIFIED**: exit 0, log "brought the running launcher's window forward", one window |
| `--check` while the settings window is open | **VERIFIED**: dialog "The Colin McRae Rally 04 launcher is already open…", exit 5 |
| First start on a fresh prefix draws the settings window (`prefer_glx_or_restart`) | **VERIFIED**: `UseEGL` absent before, `N` after; the log shows the launcher started twice, then "settings window opened"; the window drew normally (same layout as `docs/images/launcher-play.jpg`) |

Quitting from the main menu, `--look widescreen`; after each, no process
of the test was left (game, launcher, `wineserver`, gamescope, helper):

| Way | Game gone / helper gone | Helper exit |
|---|---|---|
| The game's own Quit (Escape → exit cmr04 → YES) | 0.15 s / 0.66 s | 139 (gamescope's own exit segfault, as before) |
| Alt+F4 (XTest), in the intro and in the menu | 0.31 s / 0.81 s | 139 |
| Window close (`WM_DELETE_WINDOW`, as a compositor sends) | 0.07 s / 0.58 s | 139 |
| `WM_SYSCOMMAND SC_CLOSE` | 0.45 s / 0.95 s | 139 |
| gamescope closed (SIGTERM to our gamescope) | 0.41 s / 0.66 s | 139 |
| Ctrl+C (SIGINT to the helper's group) | 0.06 s / 0.31 s | 143 |
| SIGTERM to `cmr4-play` | 0.05 s / 0.31 s | 143 |
| `cmr4-play stop` | 0.21 s / 0.22 s | 0 (stop itself 0) |

Found on the way: without a screen to ask (no `xrandr` answer), the
Widescreen look now sizes gamescope from `REA_DEFAULT_SIZE` (1024x768,
the game then picks 1024x576) where the old local hook used 1920x1080.
A desktop or a Deck always answers `xrandr`, so players shouldn't see it.

## 2026-11-02: headless real-game verification (owner's install, read-only)

Launcher at `5a51cb6` (SDK v0.3.2), release build packaged with
`packaging/build-windows.sh` into `.work/realgame/pkg`; `cmr4.exe`
SHA-256 `7dca9f4c…1b41` (FileVersion 1.0.0.0). Wine 11.21, gamescope
3.17.0 and PipeWire 1.6.10 in the `gaming` distrobox, RTX 5090.

**How it ran.** Every run went through `linux/cmr4-play --play` inside the
box under a supervisor with a finite timeout (`.work/realgame/bin/run.sh`):
a throwaway prefix, `HOME`/`XDG_*` and `XDG_RUNTIME_DIR` under
`.work/realgame/`, credentials removed from the environment, `gamescope`
wrapped to `--backend headless`, sound to a private PipeWire (own runtime
dir, WirePlumber with every hardware monitor off) with one null sink.
Input was XTest on gamescope's Xwayland (real Alt+F4 key events, a real
`WM_DELETE_WINDOW` as a compositor sends), Windows-side requests from a
small test helper (`PostMessage`, `SendInput`). Frame rates are Wine's
`WINEDEBUG=fps` trace; screenshots are `gamescopectl screenshot`. After
every run the process table was checked by name, by `WINEPREFIX`/
`XDG_RUNTIME_DIR` in `/proc/*/environ` (match only, never printed) and by
the session of the game's gamescope (gamescope has file capabilities, so
its environ is unreadable). Logs, configs and screenshots per run are in
`.work/realgame/logs/<run>/` (local, not in Git). The game folder's files
were unchanged afterwards (no file newer than the first run; hashes of
`cmr4.exe`, `OPTIONS`, `CHAMPIONSHIP_SLOT_1` and `BESTTIMES_GHOSTS`
recorded).

### Game and looks

| Check | Result | Evidence (`.work/realgame/logs/…`) |
|---|---|---|
| First launch: runtime built, game starts and renders | **VERIFIED**: 1652 files copied (~80 s from the NAS), DXVK swapchain 1920x1080, title, intro movies, attract demo, menus and Quick Race stages | `first-widescreen/`, `nav-stage/` |
| Widescreen look (16:9) | **VERIFIED**: fix applied, renders 1440x1080, `ScalingMode = stretched`, `Resolution = h:1920, v:1080` | `ws-*/`, `nav-stage/n6.jpg` |
| Original look | **VERIFIED**: unpatched, `Resolution = unforced`, `stretched_ar`, gamescope 1024x768; stage renders 4:3 | `cfg-original/`, `look-original/stage.jpg` |
| Steam Deck look | **VERIFIED**: unpatched, renders 1280x800, `FPSLimit = 60`, gamescope 1280x800 | `cfg-steamdeck/`, `look-steamdeck/stage.jpg` |
| 16:10 (`--resolution 1920x1200`) | **VERIFIED**: unpatched, renders 1920x1200, `stretched_ar` | `cfg-1920x1200/`, `res-1920x1200/stage.jpg` |
| `--look` with `--no-window` | **VERIFIED**: every run above (cmr4-play passes `--no-window`); the snapshots' values were written | as above |
| v0.2.0 `launcher.cfg` with `widescreen = off` and a 16:9 resolution | **VERIFIED**: plays with the fix (`widescreen fix applied`, renders 1440x1080) | `cfg-v020-widescreen-off/` |
| Unknown build (one copyright byte changed, SHA-256 `ab95d57e…c519`, through a symlink copy of the folder) | **VERIFIED**: log has the SHA-256 and the warning, game starts unpatched with `stretched_ar`. The window showed the message and Report button **but ran under the footer** (fixed, see below) | `unknown-16x9/`, `.work/realgame/shots/unknown-4k*.jpg` |
| Source exe already patched (`60e0267b…c0b2`, made from the owner's exe) | **VERIFIED**: 16:9 plays patched (`stretched`), Original plays unpatched (`stretched_ar`) | `prepatched-16x9/`, `prepatched-original/` |
| Command-line runs remember only `--game`; no resolution stored inside gamescope | **VERIFIED**: `launcher.cfg` gained only `game =` | `*/launcher.cfg` |
| winebus `Map Controllers` | **VERIFIED**: seeded in a fresh prefix (`seeded (absent before)`); set to 0 by hand it stays 0 | `first-widescreen`, `misc-winebus-log/`, `misc-pre.txt` |
| `launcher.log` past 1 MiB | **VERIFIED**: a 1,100,000-byte log started afresh (1766 bytes after the run) | `misc-pre.txt` |
| Sound | **NOT CHECKED**: routed to the private null sink, not recorded | — |

### Frame-rate cap and VSync (re-measured on a free GPU)

The first measurements ran while another workload used the GPU and were
discarded; these are from 15:00 on, GPU otherwise idle. Quick Race stage,
car standing, gamescope headless at 60 Hz; last ten one-second readings of
Wine's present trace.

| Setting | dgVoodoo.conf | Present mode | Frame rate | Stage timer over 7.6 s / 16.6 s of wall time |
|---|---|---|---|---|
| `--fps-limit 0 --vsync off` | `FPSLimit = 0`, `ForceVerticalSync = false` | MAILBOX | 910–970 fps | 7.63 s / 16.64 s |
| `--fps-limit 30` | `FPSLimit = 30` | MAILBOX | 29.99–30.01 fps | 7.64 s / 16.60 s |
| `--fps-limit 60` | `FPSLimit = 60` | MAILBOX | 59.96–60.07 fps | 7.62 s / 16.61 s |
| `--fps-limit 144` | `FPSLimit = 144` | MAILBOX | 143.90–144.09 fps | 7.62 s / 16.63 s |
| `--fps-limit 0 --vsync on` | `ForceVerticalSync = true` | FIFO | 59.95–60.03 fps (the 60 Hz output) | 7.64 s / 16.61 s |

- **VERIFIED:** each cap holds exactly; VSync switches DXVK to FIFO and
  locks to the output's refresh; the stage timer runs at wall-clock speed
  under every setting (within the 0.04 s reading error of the
  screenshots).
- **NOT CHECKABLE HEADLESS:** tearing (needs eyes on a real display) and
  car physics at different caps (Quick Race picks a random car and stage;
  a person should drive the same stage at 30 and at 144 fps and compare
  the time).

### Quitting (each: game and helper gone, then the process table checked)

| Way | Result | Time to game gone / helper gone |
|---|---|---|
| Alt+F4 (XTest keys) in intros, title, main menu, save-slot screens, a stage countdown | **VERIFIED** | 0.3 s / 0.8 s |
| Alt+F4 (`SendInput` from a Windows helper), main menu | **VERIFIED** | 0.7 s / 1.3 s |
| Compositor close (`WM_DELETE_WINDOW`, as Super+W) | **VERIFIED** | 0.07 s / 0.8 s |
| `WM_SYSCOMMAND SC_CLOSE` posted to `PC_CMR4` | **VERIFIED** | 0.4 s / 0.9 s |
| `WM_CLOSE` posted, and `wine taskkill /im cmr4.exe` (no `/f`, sends `WM_CLOSE`) | **Ignored by design**: the hook leaves `WM_CLOSE` alone (the intro movies use it); the game kept running | — |
| The game's own Quit (Escape → "exit cmr04" → YES) | **VERIFIED** | 0.5 s / 1.0 s |
| `cmr4-play stop` | **VERIFIED** (exit 0), next start fine | 0.13 s / 0.4 s |
| Ctrl+C (SIGINT to the helper's process group) | **VERIFIED** (exit 143) | 0.06 s / 0.8 s |
| SIGTERM to `cmr4-play` | **VERIFIED** (exit 143) | 0.06 s / 0.3 s |
| gamescope closed (SIGTERM) | **VERIFIED** | 0.1 s / 0.6 s |
| Intro movies play to the end and can be skipped (Return) without ending the game | **VERIFIED** (the first run reached the attract demo; Return skipped to the menu) | `ws-skip-menu-altf4/` |
| Steam's Stop | **NOT CHECKABLE HEADLESS** here (no Steam in the box) | — |

After every one of these the process table had no `cmr4.exe`,
`CMR4Launcher.exe`, `wineserver`, gamescope or helper of the test left.
Two notes:

- gamescope 3.17.0 itself **segfaults on exit** after its child is gone
  ("Primary child shut down!", then `Segmentation fault (core dumped)`),
  so `cmr4-play` returns 139 after an in-game quit although everything
  ended cleanly. That is gamescope's bug, not the launcher's; a core dump
  is written each time.
- One Alt+F4 out of about fifteen did nothing (save-slot screen, VSync on,
  sent right after a screenshot). Six attempts to reproduce it on the same
  screens, with and without VSync, all quit. Likely a lost XTest event in
  gamescope's input bridge (its log has "Unhandled libei event"); listed
  in case a player sees it.

### One launcher at a time (under Wine, without cmr4-play)

| Check | Result |
|---|---|
| Second `CMR4Launcher.exe` while the settings window is open | **VERIFIED**: exit 0 in 0.4 s, log "brought its window forward", one window only |
| Second `CMR4Launcher.exe --no-window` while the game runs | **VERIFIED**: dialog "Colin McRae Rally 04 is already running. Quit the game (Alt+F4 works)…", exit 1, game untouched (`second-start/second-dialog.jpg`) |
| Second `cmr4-play --play` while the game runs | **VERIFIED**: "already running", exit 0 |
| `--check` while a window is open | **VERIFIED**: refused with the "already open" message (as a dialog under Wine) |
| The game's window is not mistaken for the launcher's | **VERIFIED**: the game's top-level window is class and title `PC_CMR4`, the launcher's is titled "Colin McRae Rally 04" and the lookup also checks the process image, so the CMR3 title clash can't happen here |

### Settings window (private Xvfb, Wine at 96 DPI unless noted)

- 4K screen, 96 DPI: 1320x960 (1.5x), fully drawn. **VERIFIED.**
- 4K, Wine DPI 144: 1320x960, fully drawn. 4K, DPI 192: 1760x1280 (2x),
  fits and fully drawn. **VERIFIED** (192 gives 2x, not "about 1.5x"; it
  fits, so left as it is).
- 1280x800 screen, no `launcher.cfg`: the window opens on the **Steam Deck**
  look. **VERIFIED** under Wine; a real Deck's Desktop Mode is untested.
  With a file that has only `game =` (left by a command-line run) it opens
  on Widescreen, by design (the recommendation applies to a first run only).
- **Bug found and fixed** (`5a51cb6`): an unknown build's five-line message
  and Report button pushed the private-copy field and its note under the
  footer. Now one line beside the Report button, whose link fills the
  code into the issue form (`unknown-4k-fixed-root.jpg`).
- README screenshots retaken from this build (`9ec2e57`).

### Wine DLL overrides (rea-launcher-sdk v0.3.3)

The launcher started with plain `wine` inside a headless gamescope (no
`cmr4-play`, no `WINEDLLOVERRIDES` in its environment: `env -i` with
only `HOME`, `USER`, `PATH`, `XDG_RUNTIME_DIR`, `WINEPREFIX`, `WINEDEBUG`),
`WINEDEBUG=+loaddll`, then Alt+F4. Loader lines for the game's DLLs
(`.work/realgame/logs/plain-*-v033/loaddll.txt`):

| DLL (from the private copy) | Before (`5a51cb6`, SDK v0.3.2) | After (SDK v0.3.3, per-program overrides) |
|---|---|---|
| `d3d9.dll` (dgVoodoo 2.80) | native | native |
| `D3D11.DLL`, `DXGI.DLL` (DXVK 3.1.1) | native | native |
| `DINPUT8.dll` (Xidi) | **builtin** (Wine's, so no stick/pedal mapping) | native (it then loads Wine's `system32\dinput8.dll` itself, as `Xidi.ini` says) |
| `DSOUND.dll` (the player's DSOAL, copied from the game folder) | **builtin** | native |
| `rea_close_hook.dll` | native | native |

`launcher.log`: "Wine DLL overrides for cmr4.exe: native,builtin set for
dinput8, dsound, d3d9, d3d11, dxgi" on the first run, "already" after.
Through `cmr4-play` (which also exports `WINEDLLOVERRIDES` in the Unix
environment) all five load native as before
(`helper-after-v033/loaddll.txt`). Alt+F4 ended the game at once in all
three runs (close hook). dgVoodoo's `D3DImm.dll` is not shipped (only its
`D3D9.dll`; see "Final component and display policy"), so there is no
d3dimm to check.

### What a person still has to do

- Look at the screenshots: `.work/realgame/logs/look-original/stage.jpg`,
  `look-steamdeck/stage.jpg`, `res-1920x1200/stage.jpg`,
  `nav-stage/n6.jpg` (16:9), `.work/realgame/shots/unknown-4k-fixed-root.jpg`.
- Tearing with VSync off/on on a real display; drive one stage at 30 and
  144 fps and compare times; the pad in a fresh prefix; Steam's Stop;
  everything on native Windows (Defender, `subst` drives, console output).

## 2026-11-02: picture, closing, gamepad (development package, gaming box)

Wine 11.21 in the `gaming` distrobox, Hyprland at scale 1.5 with a portrait
4K primary (DP-2) and a landscape 4K monitor (DP-1), RTX 5090, 8BitDo
Ultimate 2 (`2dc8:310b`). Builds linked with lld (see the binutils note in
the README).

- **Portrait primary:** before the fix the game showed a 3829x682 strip
  (aspect (16/9)³) on DP-1. A probe showed Wine reporting DP-2 as primary
  `DISPLAY1` 2160x3840. With dgVoodoo `FullScreenOutput = 2` the game filled
  DP-1 (window 2560x1440 logical, fullscreen) in menus and stages.
- **Widescreen proportions:** native 16:9 render with the patch made cars too
  narrow. Rendering 2880x2160 with dgVoodoo forcing 3840x2160 `stretched`
  restored proportions (car rear 0.88 → 1.21 width/height); the user confirmed
  cars and scenery look right. The HUD stays 640x480-based and slightly wide.
- **Closing:** the hook logged `close hook installed`; the user closed the
  game with Super+W (exit 0). The first hook version also ended the game when
  the intro movie closed; it now only reacts to `SC_CLOSE`/Alt+F4 for the
  `PC_CMR4` window. Alt+F4 itself was not separately exercised.
- **Gamepad:** with the old keyboard-only Xidi mapping, menus worked but
  nothing drove. With Xidi off, buttons/hat worked and axes did not. With the
  `CMR04Gamepad` mapping and the pedals bound by pulling RT/LT, the user drove
  stages with stick steering and analogue pedals. A factory profile written by
  the launcher (log: `gamepad setup: profile now uses pad 2dc8:310b with
  analogue pedals`) drove a stage without opening Options, once the input
  device byte (0xA71AC) was included. Choosing the controller in the game's
  own menu froze the game twice; the choice was saved anyway.
- **Launcher fixes found in the field:** turning Xidi off twice failed when the
  game folder itself ships Xidi's `dinput8.dll` (now accepted as an untouched
  game file); a dgVoodoo/registry trial confirmed `OPTIONS` and saves survive
  re-syncs.
- **Settings window:** presets and the graphite/red-orange palette rendered
  under Wine; the Play/Picture screenshots in `docs/images` are from this build.

Not exercised: native Windows, Alt+F4, other pad models, Steam Deck, two pads
connected at once (Xidi reads XInput slot 0 only).

## Verified with the exact final Windows package and AppImage (v0.1.0)

### Fresh Windows-package Wine runtime

- The launcher seeded the required install registry paths.
- First launch copied **1661 files**; the second copied **0**, confirming that
  the persistent runtime was reused.
- CMR04 reached the **3840x2160 title and main menu**. The 3D attract path had
  already been reached in the preceding controlled sequence.
- The DXVK swapchain was valid at **3840x2160**, with the **RTX 5090** selected.
- The tested dgVoodoo2 2.80 mirror showed no wrapper watermark.
- Source `cmr4.exe`, `OPTIONS`, and `CHAMPIONSHIP_SLOT_1` hashes were unchanged
  after the runs.

### Raw desktop Wine

- The bounded run did not crash and produced a valid swapchain while the
  fake/borderless policy was active.
- Capture was interrupted, so this is **not visual verification** of the raw
  desktop image. No claim about the captured frame is made.

### Exact final AppImage

- Portable Wine with headless gamescope launched successfully at **3840x2160**.
- A virtual Xbox 360 uinput pad navigated title/options, gearbox, and car-class
  screens using **A and the D-pad**. This verifies the Xidi menu mapping.
- Analog steering, RT/LT pedals, and a full driven stage were not exercised.
- No task-owned processes remained after cleanup.

The settings-window capture was also correct after the final-size Wine window
policy. That is UI evidence, not evidence of a full stage.

## Earlier controlled history

1. Phase 3A started `runtime\cmr4.exe` and then crashed; it lacked DXVK.
2. dgVoodoo2 2.87.6 still crashed with registry paths and forced D3D11. It is
   not shipped or used by the final package.
3. Fresh-prefix testing established the required install paths. The launcher
   now always writes `AUDIO_PATH`, `CD_PATH`, `INSTALL_PATH`, and `VIDEO_PATH`,
   and seeds language/adapter/depth/shader/FSAA/gamma/movie defaults only when
   missing.
4. dgVoodoo2 2.80 plus DXVK then reached the title and 3D attract demo after
   those paths were seeded.

A screenshot from another session was not attributable to the failed run and
is excluded from evidence.

## Final component and display policy

- Only the tested dgVoodoo2 2.80 `D3D9.dll` is shipped, with SHA-256
  `6f0eae6d0fbe836490b1aa3361304d2398fdb5d815a929e126b5ae7b13b16e47`.
  Provenance is in `packaging/mirror/README.md`; the archive now lives in
  the repository's `thirdparty` release. D3D8/D3DImm/DDraw are not
  installed, and the watermark is disabled.
- DXVK 3.1.1 x32 `d3d11.dll` and `dxgi.dll` install only under Wine/Proton.
- Default resolution is the largest exact 16:9 fit to the active landscape
  screen, capped at 3840x2160; unknown screens use 1920x1080. Explicit custom
  values persist. The widescreen fix follows the resolution (16:9 only).
- Raw Wine uses borderless dgVoodoo fullscreen. Native Windows and gamescope
  use normal fullscreen. A portrait primary monitor is avoided with dgVoodoo
  `FullScreenOutput`. Widescreen renders 4:3 and is stretched to 16:9;
  other modes keep their aspect with `stretched_ar`.
- Wine creates settings windows at final scaled size and seeds `UseEGL=N` only
  if missing. Native Windows keeps high-performance GPU preference.
- Linux exports both SDL background-event variables independently and keeps
  existing user choices.

## Unreleased changes awaiting a game check

Most items below were checked headless on 2026-11-02; results in
"headless real-game verification" at the top. What is still open there
is listed under "What a person still has to do".

Host tests and the Windows cross-build cover these; none has been exercised
in the game yet.

- **winebus `Map Controllers`** is now seeded only when absent (it was forced
  to 1 every launch). Check that a fresh prefix still drives with the pad, and
  that a prefix where it was set to 0 by hand keeps 0.
- **`launcher.cfg` writing** keeps comments and unknown lines, and command-line
  runs remember only `--game`. Check that Play in the window under Wine and on
  Windows still saves every picture value, and that `cmr4-play --play` after a
  window save keeps using the saved resolution for gamescope. Outside
  gamescope a first run with no saved resolution stores the one it picked;
  inside gamescope it stores none (a `--resolution` flag is never stored).
- **First run on a Steam Deck** now opens on the Steam Deck look (1280x800,
  widescreen off, 60 fps) instead of 4K (1280x720 widescreen). The look itself
  is unchanged; check that the window sees the Deck's 1280x800 screen in
  Desktop Mode.
- **`cmr4-play stop`** now also runs `wineserver -k` on the private prefix.
  Check with a running stage that the game, its wineserver and the AppImage
  exit cleanly and that the next start works.
- **Atomic writes on Windows** retry a denied rename 5 × 100 ms (antivirus
  holding the fresh patched `cmr4.exe`). Check a first launch on Windows with
  Defender active.
- **Folders that can't be canonicalized** (RAM disk, `subst`) now fall back
  to lexical normalisation. Check a game or runtime folder on such a drive on
  Windows.
- **Settings window with Wine's DPI at 144 and 192** (`winecfg` → Graphics)
  on a 4K screen: it must fit the screen at about 1.5x and be fully drawn;
  at 96 DPI it must still open at 1.5x.

### rea-launcher-sdk v0.2.0 pass (2026-11-02)

Most items below were checked headless on 2026-11-02; results in
"headless real-game verification" at the top. What is still open there
is listed under "What a person still has to do".

Host tests, the GUI tests (headless, host and Wine) and the Windows build
cover these; none has been run in the game yet.

- **Frame-rate cap and VSync have never been checked in the game.** Both
  only write dgVoodoo's `FPSLimit` / `ForceVerticalSync`. Check with an FPS
  overlay that 30/60/144 cap the game, that VSync removes tearing, and
  that a cap doesn't change car physics or the stage timer. Until then
  they stay as they were (recommendation to the owner in the pass report).
- **Widescreen follows the resolution.** A 16:9 resolution always applies
  the fix (rendering 4:3, stretched); 4:3/16:10/ultrawide and Original run
  unpatched. Check one 16:9 and one 16:10 stage; check that a v0.2.0
  `launcher.cfg` with `widescreen = off` and a 16:9 resolution now plays
  with the widescreen view.
- **Unknown builds play unpatched.** With an exe the launcher doesn't know
  (any byte changed outside the fix), check that the window says so with
  the Report button, the log has the SHA-256, and the game starts with the
  game's own view and `ScalingMode = stretched_ar`.
- **The source exe already carrying the fix** (`60e0267b…`, the owner's
  earlier copies): the launcher now rebuilds the runtime exe from the
  identified pristine bytes each launch. Check that 16:9 still plays
  patched and Original unpatched from such a source.
- **`--look`** (original, widescreen, steam_deck) with `--no-window`.
- **Windows console:** `CMR4Launcher.exe --help` and `--check` from
  `cmd.exe` print in the terminal; a double-click with a bad
  `launcher.cfg` still shows a dialog.
- **Settings window:** the reworked Picture tab, the Widescreen look card
  and Ctrl+Tab between tabs were only tested headless; look at them once
  in a real window under Wine and on Windows (the screenshots in
  `docs/images/launcher-*.jpg` were retaken on 2026-11-02 from this build,
  headless: Xvfb at 3840x2160, Wine at 96 DPI, window at 1.5x).
- **`launcher.log`** starts afresh past 1 MiB (it never did before).

### rea-launcher-sdk v0.3.0 bump (2026-11-02)

Most items below were checked headless on 2026-11-02; results in
"headless real-game verification" at the top. What is still open there
is listed under "What a person still has to do".

Host tests, the Windows-target tests under Wine and the release build
cover these; none has been run with the game yet.

- **Linked with LLD.** GNU ld had left kernel32 imports of the exe
  unresolved; `check-imports.sh` now reports none. Start the packaged
  launcher once on Windows and once under Wine and play a stage.
- **Close hook from the SDK** (`rea_close_hook.dll`,
  `REA_CLOSE_HOOK=class=PC_CMR4;strategy=terminate`, replacing
  `cmr4close.dll`). Check every way of quitting with the real game,
  headless (`gamescope --backend headless`), and that nothing is left
  afterwards (no `cmr4.exe`, launcher, `wineserver` or gamescope):
  - Alt+F4 in a stage and in the menus;
  - the window's close / the compositor's close (Super+W), which Wine
    turns into `SC_CLOSE`;
  - the intro movies still play to the end and can be skipped without
    ending the game (they are closed with `WM_CLOSE`);
  - the game's own Quit;
  - `cmr4-play stop`, Ctrl+C / SIGTERM to `cmr4-play`, Steam's Stop, and
    closing gamescope.
- **One launcher at a time.** A second `CMR4Launcher.exe` while the
  settings window is open must bring that window forward and exit; while
  the game runs it must say the game is already running and not start a
  second copy. Check on Windows and under Wine (without `cmr4-play`, whose
  session lock covers Linux). The launcher has no stop path of its own
  (it waits for the game), so `Running::close_gracefully` is not used.

## Remaining unknowns

- **Native Windows:** startup, dgVoodoo-to-system-D3D11 behavior, native GPU
  preference, and clean exit are untested.
- **Driving coverage:** stick steering and analogue RT/LT pedals were driven
  in stages on 2026-11-02 with one 8BitDo pad; other pads and two-pad setups
  are untested.
- **Steam Deck / Proton:** no separate result beyond the verified Wine and
  AppImage paths.
- **Raw desktop visuals:** capture was interrupted; only no-crash and valid
  swapchain observations are established.
- **Protected retail media:** compatibility remains unknown. Older secondary
  reports indicate it may not run under Wine; there is no project result.
- **Executable variants:** only the owner's build (and its patched form)
  is identified. Its version resource says 1.0.0.0, but it was compiled
  on 12 May 2004 (`__DATE__` string "May 12 2004 16:52:47"), after the
  2 April 2004 retail release and a month before patch 1.01 (16-17 June
  2004), so it is most likely the 1.01 exe, which kept the 1.0.0.0
  version resource; see `RESEARCH.md`. The original retail exe and other
  regional or budget builds are unknown and run unpatched until reported.
- **Old-game trouble spots** surveyed in `RESEARCH.md` but not run: EAX
  without DSOAL, behaviour on focus loss, WMV intros on a Wine without
  GStreamer.

## Evidence for future reports

Record launcher/package version, OS/Wine/Proton version, GPU, game edition,
executable SHA-256 (never the executable), settings without personal paths,
observed results, and `launcher.log`. Associate screenshots and process
identity with the same owned session. Do not attach game assets or saves.
