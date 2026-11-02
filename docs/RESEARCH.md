# CMR04 research and component rationale

## Established recipe and executable boundary

The launcher uses the tested CMR04 setup rather than generic CMR1 options:

- `cmr4.exe` is PE32. The 16:9 Low patch replaces exactly one occurrence of
  `04 00 00 80 3f e8` with `04 00 00 40 3f e8`, only in the private runtime.
- **What the patch does** (function at 0x477820): every camera gets two
  numbers, a horizontal factor (the patched constant, 1.0 → 0.75) and the
  render's width/height. The 3D projection scales horizontally by the first
  and vertically by the second, so the patch is correct for a **4:3 render
  stretched to 16:9**. Rendering 16:9 natively applies the 16:9 correction
  twice and squeezes the view by 0.75. With widescreen on, the launcher
  therefore writes a 4:3 game mode (height × 4/3, e.g. 2880x2160) and has
  dgVoodoo force the 16:9 output with `ScalingMode = stretched`. Measured on a
  car's rear in the same scene: width/height 0.88 before, 1.21 after (×1.37,
  expected 1/0.75 = 1.33).
- The HUD is drawn on a fixed 640x480 grid (main loop scales by width/640 and
  height/480), so it always fills the screen and looks slightly wide at 16:9.
  Correcting it would need a second code patch; not done.
- Width and Height are written every launch. Widescreen requires exact 16:9;
  Original, 4:3, 16:10, and ultrawide use widescreen off (unpatched CMR04
  keeps correct proportions at any aspect).
- The default is the largest exact 16:9 mode fitting the chosen monitor,
  capped at 3840x2160. Unknown screens use 1920x1080.
- CMR04 requires install path strings and first-run defaults in its 32-bit
  registry key. They are seeded without exposing language/graphics options.
- **Monitor:** the game sizes its window from the primary display and dgVoodoo
  presents on the adapter's default output. A portrait primary monitor (seen:
  Wine `DISPLAY1` 2160x3840) produced a full-width strip (9/16)² of the screen
  height. `display::choose` keeps a landscape primary, otherwise passes the
  largest landscape monitor as dgVoodoo `FullScreenOutput` (1-based,
  enumeration order). Raw Wine still uses `FullscreenAttributes = fake`
  (borderless); gamescope and native Windows use normal fullscreen.
- **Closing:** the window procedure (0x501EF0) returns 0 for `WM_CLOSE` and
  swallows Alt-key presses other than Tab/Enter; the main loop (0x503000)
  ignores `WM_QUIT`. The launcher installs a `WH_GETMESSAGE` hook from
  `cmr4close.dll` on the game's main thread that ends the process on
  `SC_CLOSE` or Alt+F4 aimed at the `PC_CMR4` window. Plain `WM_CLOSE` is
  ignored because the intro movie's window receives one on the same thread.
- **Gamepad:** CMR04 reads a DirectInput joystick, requests X/Y/Z/RX/RY/RZ, a
  hat and buttons, and sets every axis to 0..255. Analogue pedals use one axis
  for accelerate and brake. Xidi's `CMR04Gamepad` keeps StandardGamepad's
  layout (stick X, hat, buttons 1–12), puts RT/LT on that pedal axis (Y, shown
  as "Z axis" in the game's menu), ignores the left stick's Y, and maps no
  keyboard keys. The earlier keyboard-only mapping hid the pad's axes from the
  game. Without Xidi, Wine's own pad worked for buttons and hat but its axes
  did not drive in our tests.
- **Profile (`OPTIONS`, 987,056 bytes, no checksum):** see `src/profile.rs`.
  Player 1's input device type is at 0xA71AC (0 keyboard, 2 controller); the
  controller block at 0x4AA4 holds steering/pedal modes, button bindings, the
  pedal axis, the pad's USB product/vendor IDs (0x4B18) and axis tuning
  (0x4BE0); players 2–4 repeat it every 0x29C40 bytes. The game picks the pad
  whose IDs match at startup. A factory profile that the launcher filled in
  matched, byte for byte, the file the game saved after the same setup by hand,
  and drove a stage without opening Options.

References:

- [PCGamingWiki: Colin McRae Rally 04](https://www.pcgamingwiki.com/wiki/Colin_McRae_Rally_04)
- [Xidi](https://github.com/samuelgr/Xidi)
- [dgVoodoo2 official project](https://dege.freeweb.hu/dgVoodoo2/dgVoodoo2/)
- [Dege's official dgVoodoo2 repository](https://github.com/dege-diosg/dgVoodoo2)
- [DXVK](https://github.com/doitsujin/dxvk)

The in-game Advanced Graphics menu rewrote the tested resolution and hung;
users are advised to avoid it. A 1920x1440 fallback also failed in the recipe
and is not presented as known-good.

## Controlled runtime evidence

The exact sequence that determined the final policy:

1. dgVoodoo2 2.87.6 crashed even after required registry paths were present
   and D3D11 was forced. It is not used anywhere in the final package.
2. Fresh-prefix testing showed that CMR04 also needed `AUDIO_PATH`, `CD_PATH`,
   `INSTALL_PATH`, and `VIDEO_PATH` pointed at the private runtime. The
   launcher now always writes those paths and seeds game defaults only if
   missing, preserving player choices thereafter.
3. dgVoodoo2 2.80 plus DXVK reached CMR04's title and 3D attract demo at 4K
   after those paths were seeded. The measured swapchain was 3840x2160.
4. The exact final package copied 1661 files on its first launch and zero on
   its second. It reached the 4K title/main menu on an RTX 5090 without a
   watermark, and source executable/settings hashes remained unchanged.
5. Settings-window capture was correct after final-size creation under Wine.
6. The exact AppImage reached 4K title/options, gearbox, and car-class screens
   with a virtual Xbox pad using A and the D-pad, verifying Xidi menu input.
   Analog steering/pedals and a full stage were not exercised.
7. Raw desktop Wine did not crash and had a valid swapchain, but its capture
   was interrupted and is not visual verification. Native Windows, a full
   stage, separate Proton/Deck, and protected retail media remain unknown.

A screenshot from another session was not attributable to the failed run and
was excluded from evidence.

## Compatibility pins and narrow mirror

`packaging/components.pins` records official downloads. Archives are cached,
SHA-256 checked, and exact members are installed. No compatibility binary is
taken from the installed Codemasters game.

### dgVoodoo2 2.80

The official v2.8 release has only the unrelated dgVoodoo API archive; the
full 2.80 binary distribution is unavailable from Dege. The documented narrow
exception is `packaging/mirror/dgVoodoo2_80.zip`, containing only the tested
third-party `MS/x86/D3D9.dll` and no game data:

- DLL SHA-256: `6f0eae6d0fbe836490b1aa3361304d2398fdb5d815a929e126b5ae7b13b16e47`
- ZIP SHA-256: `cec7e2c8eeaec0065cc8f41d8e0f176ae28d55074c4295ce3670589dc9f29d30`
- Provenance and official links: `packaging/mirror/README.md`

CMR04 imports D3D9 and the working test needs only this dgVoodoo DLL. Unused
D3D8.dll, D3DImm.dll, and DDraw.dll are not packaged or installed. The
configuration disables the dgVoodoo watermark.

### Xidi 4.3.1

- URL: <https://github.com/samuelgr/Xidi/releases/download/v4.3.1/Xidi-v4.3.1.zip>
- SHA-256: `78120f44baa816fc264fb722495776aab437a36be515e42345428b7f32ac17ff`
- Payload: `Xidi-v4.3.1/Win32/dinput8.dll` plus license notices.

Xidi 5.0 is deliberately not adopted without a required feature or CMR04
runtime evidence.

### DXVK 3.1.1

- URL: <https://github.com/doitsujin/dxvk/releases/download/v3.1.1/dxvk-3.1.1.tar.gz>
- SHA-256: `40565b4a724aadc4433fa4e010b4b23916d9b1f1baeee64e17186db94f54e608`
- Payloads: `dxvk-3.1.1/x32/d3d11.dll` and `dxgi.dll`, only under Wine/Proton.
- License: zlib/libpng from the official v3.1.1 repository.

Native Windows does not require or install DXVK and retains dgVoodoo to system
D3D11. Ownership/source-byte conflict checks apply to every wrapper.

## Display and Linux details

CMR3's measured Wine results showed exclusive fullscreen selecting a portrait
monitor and cropping output, while borderless or gamescope worked. CMR04 uses
`FullscreenAttributes=fake` only for raw Wine. Borderless alone was not enough
for CMR04: with a portrait primary monitor the picture was still fitted to it,
so the launcher also sets dgVoodoo's `FullScreenOutput` (see above). Under
gamescope there is a single output and the default is kept.
`GAMESCOPE_WAYLAND_DISPLAY` selects context without adding a user option.

Wine's settings window seeds `UseEGL=N` only if absent and is created at its
final scaled size. Linux passes both SDL controller and joystick background
event variables independently, preserving existing user values.

See [RUNTIME_NOTES.md](RUNTIME_NOTES.md) for remaining verification.
