# Runtime verification notes

This page distinguishes controlled runtime evidence from coverage that is
still unknown. Portable tests and package inspection support the implementation
but are not treated as gameplay evidence.

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
  Provenance is in `packaging/mirror/README.md`. D3D8/D3DImm/DDraw are not
  installed, and the watermark is disabled.
- DXVK 3.1.1 x32 `d3d11.dll` and `dxgi.dll` install only under Wine/Proton.
- Default resolution is the largest exact 16:9 fit to the active landscape
  screen, capped at 3840x2160; unknown screens use 1920x1080. Explicit custom
  values persist. Other aspect ratios require widescreen off.
- Raw Wine uses borderless dgVoodoo fullscreen. Native Windows and gamescope
  use normal fullscreen. A portrait primary monitor is avoided with dgVoodoo
  `FullScreenOutput`. Widescreen renders 4:3 and is stretched to 16:9;
  other modes keep their aspect with `stretched_ar`.
- Wine creates settings windows at final scaled size and seeds `UseEGL=N` only
  if missing. Native Windows keeps high-performance GPU preference.
- Linux exports both SDL background-event variables independently and keeps
  existing user choices.

## Unreleased changes awaiting a game check

Host tests and the Windows cross-build cover these; none has been exercised
in the game yet.

- **winebus `Map Controllers`** is now seeded only when absent (it was forced
  to 1 every launch). Check that a fresh prefix still drives with the pad, and
  that a prefix where it was set to 0 by hand keeps 0.
- **`launcher.cfg` writing** keeps comments and unknown lines, and command-line
  runs remember only `--game`. Check that Play in the window under Wine and on
  Windows still saves every picture value, and that `cmr4-play --play` after a
  window save keeps using the saved resolution for gamescope (a `--play` run
  no longer stores the resolution it picked).
- **First run on a Steam Deck** now opens on the Steam Deck look (1280x800,
  widescreen off, 60 fps) instead of 4K (1280x720 widescreen). The look itself
  is unchanged; check that the window sees the Deck's 1280x800 screen in
  Desktop Mode.
- **`cmr4-play stop`** now also runs `wineserver -k` on the private prefix.
  Check with a running stage that the game, its wineserver and the AppImage
  exit cleanly and that the next start works.

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
- **Executable variants:** original unpatched PE32, already patched source
  restoration, and protected-edition coverage remain to be exercised.

## Evidence for future reports

Record launcher/package version, OS/Wine/Proton version, GPU, game edition,
executable SHA-256 (never the executable), settings without personal paths,
observed results, and `launcher.log`. Associate screenshots and process
identity with the same owned session. Do not attach game assets or saves.
