# Contributing findings back

Drafts of what this launcher found that the wider community could use:
PCGamingWiki first, then upstream projects. **The owner reviews and posts;
agents only draft** (`rea-launcher-sdk/docs/playbooks/contribute-back.md`).
Nothing here has been posted. When a draft is posted, record the link and
the date in its status.

These are copied from the SDK's collection
([`docs/CONTRIBUTIONS.md`](https://github.com/akitaonrails/rea-launcher-sdk/blob/main/docs/CONTRIBUTIONS.md),
items C3–C6, prepared 2026-11-02) and kept in step with this repository.
Before posting, the owner:

1. checks the live page again (the survey was read through Wayback
   snapshots, since PCGamingWiki blocks automated clients);
2. fills in `{{Refcheck|user=…}}` with their user name and the date;
3. checks each hex edit on a copy of the exact executable named in the
   draft (same SHA-256), never on another build;
4. never attaches game files, patched executables or saves.

WineHQ AppDB takes plain-Wine results only, so the wrapper-based Wine
findings (C6) go to a PCGamingWiki Linux note instead.

| # | Finding | Target | Status |
|---|---|---|---|
| C3 | AuToMaNiAk005's 16:9 fix is meant for a 4:3 render stretched to 16:9 | PCGamingWiki, Widescreen fixbox (`{{ii}}` note) | ready, unless the fix's readme already says so (owner to check); not posted |
| C4 | CMR04 ignores Alt+F4 and window-close requests | PCGamingWiki, Issues unresolved | ready for Linux/Wine; needs a Windows check before claiming it there; not posted |
| C5 | Analogue pedals on the triggers with Xidi | PCGamingWiki, Input fixbox | ready; not posted |
| C6 | Wine/Proton: registry paths, dgVoodoo2 2.80 + DXVK, portrait primary monitor | PCGamingWiki, Linux note | ready; not posted |

The registry key for C6 (`src/registry.rs`): `HKEY_LOCAL_MACHINE\SOFTWARE\Codemasters\Colin McRae Rally 04`
in the 32-bit view (`SOFTWARE\WOW6432Node\…` on 64-bit Windows), with the
string values `AUDIO_PATH`, `CD_PATH`, `INSTALL_PATH` and `VIDEO_PATH`.

The patch table for C3 comes from `patch::describe(&edition::WIDESCREEN)`
(see `docs/RESEARCH.md`, "Editions"); don't retype offsets by hand.

### C3 — Colin McRae Rally 04: use a 4:3 mode with the 16:9 fix

Page: <https://www.pcgamingwiki.com/wiki/Colin_McRae_Rally_04>, the "Use
widescreen fix" fixbox (AuToMaNiAk005's fix with dgVoodoo 2). Its "16:9
Low" option changes exactly one float in `cmr4.exe`, the camera's
horizontal factor 1.0 → 0.75:

- file offset **077829**: `80` → `40` (in `C7 44 24 04 00 00 80 3F E8`, i.e.
  `mov dword [esp+4], 1.0f` → `0.75f`; function at VA 0x477820);
- measured on `cmr4.exe` 3,756,032 bytes, FileVersion 1.0.0.0, SHA-256
  before the edit `7dca9f4cab16bc1a98df8871f3d20aec4cd931a818ea0899db86c83c1f712b41`
  (derived by reverting that one byte in the launcher's runtime copy; the
  launcher changes no other byte. PCGW says the fix "requires 1.01", yet the
  version resource reads 1.0.0.0. The owner should check which patch level
  this copy is).

Suggested `{{ii}}` line to add inside that fixbox (do not post a separate
hex edit, since the bytes are the community fix):

```wikitext
{{ii}} The 16:9 setting assumes a 4:3 picture stretched to 16:9. Set a 4:3 resolution in the game (e.g. 1440x1080 for a 1920x1080 screen, 2880x2160 for 3840x2160) and set dgVoodoo's scaling mode to ''Stretched''; at a native 16:9 resolution cars and scenery come out about 25% too narrow. The HUD remains drawn on a 640x480 grid and looks slightly wide.<ref>{{Refcheck|user=OWNER|date=2026-11-02|comment=Measured on a car's rear: width/height 0.88 at native 16:9, 1.21 with a 4:3 render stretched. https://github.com/akitaonrails/rea-colin-mcrae-4/blob/master/docs/RESEARCH.md}}</ref>
```

Verified in game (RUNTIME_NOTES 2026-11-02, user-confirmed). Before
posting, check whether the fix's own readme already says to use a 4:3 mode.
If it does, this is "already documented" and the line is unnecessary.

### C4 — Colin McRae Rally 04: Alt+F4 does nothing

```wikitext
===Game cannot be closed with Alt+F4 or the window's close button===
{{ii}} The game's window procedure ignores close requests and its main loop ignores <code>WM_QUIT</code>, so Alt+F4, the close button (and a window manager's close action under Linux) do nothing. Quit from the main menu, or end {{file|cmr4.exe}} with Task Manager.<ref>{{Refcheck|user=OWNER|date=2026-11-XX|comment=Observed under Wine; analysis and a close hook: https://github.com/akitaonrails/rea-launcher-sdk/tree/main/crates/rea-launcher-close-hook}}</ref>
```

Status: observed under Wine (Super+W did nothing before the hook). The
cause is in the game's own WndProc (0x501EF0 returns 0 for `WM_CLOSE`) and
main loop (0x503000), so it should apply on Windows too, **but that is
untested**. Check Alt+F4 on Windows before posting it as a general issue;
otherwise post it under the Linux note only. Also note: in the hook's
testing Alt+F4 itself was not separately exercised (SC_CLOSE was).

### C5 — Colin McRae Rally 04: analogue pedals on the triggers (Xidi)

PCGW's Input rows point at the "force feedback restore module" for XInput.
Here is an alternative verified fixbox for analogue triggers:

```wikitext
{{Fixbox|description=Use Xidi for analogue trigger pedals|ref=<ref>{{Refcheck|user=OWNER|date=2026-11-02|comment=Xidi 4.3.1, 8BitDo Ultimate 2; full stages driven.}}</ref>|fix=
{{ii}} The game reads one DirectInput axis for both pedals. This mapping puts RT/LT on that axis and keeps the stick and buttons in the standard layout.
# Download [https://github.com/samuelgr/Xidi/releases Xidi] and copy its 32-bit {{file|dinput8.dll}} next to {{file|cmr4.exe}}.
# Create {{file|Xidi.ini}} next to it with:
<pre>
[Mapper]
Type = CMR04Gamepad

[CustomMapper:CMR04Gamepad]
Template = StandardGamepad
StickLeftX = Axis(X)
StickLeftY = Null
TriggerRT = Axis(Y, -)
TriggerLT = Axis(Y, +)
</pre>
# In ''Options → Controls'', choose the controller, set analogue pedals and bind them by pulling RT/LT (the game's menu shows the pedal axis as "Z axis").
}}
```

Notes: tested only with Xidi 4.3.1 and one pad. The launcher's full INI also
sets 5% stick and 0% trigger deadzones and `ActiveVirtualControllerMask = 0x01`;
the lines above are the part that matters. Choosing the controller in the
game's menu froze the game twice under Wine, although the choice was saved.
Worth a `{{--}}` line if the owner sees it on Windows too.

### C6 — Colin McRae Rally 04: Wine/Proton (PCGW Linux note)

Not eligible for AppDB 2751 (the working setup uses the dgVoodoo/DXVK
wrappers). Turn this into a PCGW "Linux" `{{ii}}`/fixbox, or keep it as
reference text:

> Wine 11.20/11.21 (WoW64). Plays at 4K with dgVoodoo2 **2.80** `D3D9.dll`
> (MS/x86) + DXVK 3.1.1 x32 `d3d11.dll`/`dxgi.dll`; dgVoodoo2 2.87.6
> crashed. A fresh prefix needs `AUDIO_PATH`, `CD_PATH`, `INSTALL_PATH`
> and `VIDEO_PATH` in the game's registry key (32-bit view) pointing at the
> install folder, or the game crashes at start. With a portrait primary
> monitor the picture is a thin strip: set dgVoodoo's `FullScreenOutput`
> to the landscape monitor. Without a wrapper (Xidi), Wine's pad gives
> buttons and hat but no working axes. Alt+F4 does nothing (see above).
> Avoid the in-game Advanced Graphics menu: it rewrote the resolution and
> hung.

Before posting, take the exact registry key path from
`rea-colin-mcrae-4/src/registry.rs`; it is not repeated here. The
Advanced Graphics hang comes from the earlier recipe, not a dated run.
Drop it if the owner cannot reproduce it.

