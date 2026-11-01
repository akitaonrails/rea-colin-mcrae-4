# Runtime verification notes

This page distinguishes controlled runtime evidence from coverage that is
still unknown. Portable tests and package inspection support the implementation
but are not treated as gameplay evidence.

## Verified with the exact final Windows package and AppImage

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
  use normal fullscreen. Aspect is preserved with `stretched_ar`.
- Wine creates settings windows at final scaled size and seeds `UseEGL=N` only
  if missing. Native Windows keeps high-performance GPU preference.
- Linux exports both SDL background-event variables independently and keeps
  existing user choices.

## Remaining unknowns

- **Native Windows:** startup, dgVoodoo-to-system-D3D11 behavior, native GPU
  preference, and clean exit are untested.
- **Driving coverage:** analog steering, RT/LT pedals, and a full stage have
  not been tested. A and D-pad menu navigation is not analog-driving evidence.
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
