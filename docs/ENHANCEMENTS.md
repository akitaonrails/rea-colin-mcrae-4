# CMR04 graphics enhancement research

Practical, opt-in ways to improve the image without confusing post-processing
with renderer features CMR04 does not have. This is a design recommendation,
not an implemented launcher option or a promise of compatibility.

## Why CMR04 looks flat

CMR04 is a legacy Direct3D 9 renderer. Its lighting and material model predate
modern per-pixel PBR, and its soft vehicle/ground contact shadows use textures
and blur passes. REA string observations such as `ShadowBlurTex` and
`CarShadowSoften`, plus its custom shader manager, are useful investigation
leads. They do not establish a modern G-buffer, normals, material IDs, or
motion vectors suitable for modern GI or temporal rendering.

The result can look evenly lit and weightless even at high resolution. Native
3840x2160 already works through the tested dgVoodoo2 2.80 + DXVK path, so more
pixels are not the remaining problem. Contact shading, grading, and selective
sharpness are more plausible improvements.

Screen-space ambient occlusion (SSAO) is plausible only if the actual CMR04
depth buffer can be captured correctly. True global illumination cannot be
enabled with a configuration switch. SSDO, a coarse first bounce, or RTGI would
be screen-space approximations with different artifacts, not true GI.

## Recommended implementation ladder

### 1. Colour-only post-processing: safest first candidate

Start with effects that need no scene depth:

- CAS or similarly restrained sharpening;
- a subtle LUT/colour grade;
- optional debanding;
- conservative exposure/contrast adjustment rather than bloom-heavy grading.

These can improve clarity without inventing geometry or shadow data. They are
also the fallback when depth capture is absent or wrong. Keep the effect subtle
and compare against native 4K rather than treating stronger as better.

Optional in-engine or dgVoodoo anisotropic filtering is a separate, relatively
low-risk candidate after visual tests. It should be evaluated independently of
post-processing so its benefit and any artifacts remain attributable.

### 2. Cinematic: colour plus subtle SSAO, only after a depth proof

After proving depth, add small-radius SSAO at low intensity, potentially at
half resolution to control cost and noise. The desired effect is restrained
contact shading under cars and near trackside objects, not a dark outline
around the whole scene.

SSDO, a coarse first bounce, or RTGI can remain a later experiment. They must
be described as approximations and must not silently become the default.
Menus, FMV, HUD, and results screens need masking or an easy effects toggle;
depth effects intended for the 3D scene must not dirty interface elements.

A later project-authored SSAO implementation may be preferable for a
self-contained package: provenance, redistribution, tuning, and support would
be under project control. That is engineering work, not a preset download.

### 3. Lower-priority renderer investigations

Changing shadow blur behavior through REA is less attractive than getting a
safe post-processing path first. It requires exact instruction/state analysis
and carries regression risk. Do not confuse shadow texture strings with proof
that modern lighting can be injected.

RTX Remix is a separate manual Windows/NVIDIA remaster project involving asset
capture, replacement, and scene/light authoring. It is not a launcher option or
a small compatibility preset.

## Concrete launcher design

Use one future setting only:

```text
postfx = off | color | cinematic
```

The default is **off**, preserving the verified faithful path.

- `off`: install or enable no post-processing layer and apply no shader config.
- `color`: enable only the depth-independent colour preset.
- `cinematic`: enable the colour preset plus subtle SSAO, but only after depth
  capture has passed an exact-game validation. If depth is unavailable or
  invalid, fail visibly or fall back to `color` with a clear warning. Never
  claim AO is active when it is not.

### Windows

Use ReShade as `dxgi.dll` to hook dgVoodoo's D3D11 output. Do **not** install it
as `d3d9.dll`: dgVoodoo2 2.80 owns that slot. DXVK's Wine `d3d11.dll` and
`dxgi.dll` ownership must also remain explicit; do not overwrite them blindly.
ReShade itself is BSD-3-Clause, but every shader/preset needs separate review.

qUINT MXAO is useful for a local proof of AO or an approximate first-bounce
look. Its official license is all rights reserved, so do not bundle qUINT or a
derivative preset without permission. A user-supplied research preset is not
the zero-hassle final product and should not become a silent download
dependency.

### Linux

Use vkBasalt over DXVK rather than adding more proxy DLLs. This avoids the
existing dgVoodoo/DXVK/Xidi slots and fits the AppImage/gamescope path.
Official vkBasalt documents `depthCapture = on` as WIP, warns that the selected
buffer may be wrong, and notes platform-specific problems. Cinematic therefore
needs a depth proof here too, not merely a successful layer load.

Apply CMR3's manifest lessons, not an assumption that its AO works: CMR3's
current real postfx is CAS-only. Vulkan layer manifests should use absolute
paths with the correct 32/64-bit library for the target, and stale manifests
must be removed when postfx is off so an old layer cannot reactivate itself.

## Risks and verification plan

1. Capture a native-4K baseline with the exact final package: title, menus,
   attract mode, and a driven stage when available.
2. Test `color` alone. Compare shimmer, banding, contrast, HUD legibility, and
   frame pacing against that baseline.
3. Visualize the captured depth buffer before enabling AO. Check road/car
   separation, distance ordering, transparency, resolution, and fullscreen
   versus borderless/gamescope paths.
4. If depth is wrong, stop the cinematic test. Do not use a screenshot with AO
   enabled as proof that AO sampled the right buffer.
5. If depth is valid, test subtle SSAO at small radius and then half-res if
   needed. Look for halos, over-dark cars, flicker, and temporal instability.
6. Test menus, FMV, HUD, pause/results, and race separately. Add masking or a
   toggle if interface output is affected.
7. Repeat on native Windows, raw Wine, AppImage, and Proton/Deck separately.
   A working Linux layer says nothing about native Windows compatibility.
8. Verify off really unloads the layer and leaves no stale manifest, generated
   config, DLL, or environment override behind.
9. Record exact package/library/shader versions, licenses, screenshots, GPU,
   resolution, and log evidence. Do not infer GI, depth, or frame-rate results
   from appearance alone.

## DLSS 5 assessment

NVIDIA's developer material describes DLSS 5 as a final neural-rendering stage
using rendered frame colour and motion vectors, initially for RTX 50 series.
Richer engine data improves its results. CMR04 does not provide that data
natively, and native 4K already needs no upscaling.

The community DLSS5-Feeder makes a 32-bit D3D9 route through dgVoodoo and a
64-bit helper technically possible; "impossible" would be inaccurate. That is
not a production recommendation:

- it is beta and relies on estimated motion vectors rather than CMR04 engine
  integration;
- ghosting/smearing and HUD processing are expected risks;
- proprietary NVIDIA DLLs and neural add-ons, some distributed only through
  Discord, complicate provenance and redistribution;
- driver/model compatibility is fragile and Linux reports are limited;
- packaging and license review would be substantial even before exact-game
  visual validation.

Do not integrate DLSS 5 by default or in the near term. Revisit only as a later
experimental opt-in after exact-game validation, packaging provenance, and
licensing are established. No frame-generation recommendation: it would add
latency and complexity around a legacy game that already runs at native 4K.

## Sources and evidence

- NVIDIA, DLSS 5 developer article:
  https://developer.nvidia.com/blog/whats-new-for-game-developers-dlss-5-with-3d-guided-neural-rendering-nvidia-ace-updates-and-new-rtx-kit-capabilities/
- Community DLSS5-Feeder:
  https://github.com/jlrouzies-fr/DLSS5-Feeder
- ReShade and its BSD-3-Clause license:
  https://reshade.me/ and https://github.com/crosire/reshade/blob/main/LICENSE.md
- vkBasalt, configuration, shader support, and depthCapture warnings:
  https://github.com/DadSchoorse/vkBasalt
- qUINT/MXAO and its license:
  https://github.com/martymcmodding/qUINT
- RTX Remix:
  https://github.com/NVIDIAGameWorks/rtx-remix
- CMR3 enhancement research and vkBasalt manifest lessons:
  https://github.com/akitaonrails/rea-colin-mcrae-3/blob/main/docs/ENHANCEMENTS.md
- CMR04 controlled runtime results and current display policy:
  `docs/RUNTIME_NOTES.md`
- CMR04 component/display research:
  `docs/RESEARCH.md`

Local REA observations identify shadow-related strings and a custom shader
manager; they are leads for later analysis, not evidence of a G-buffer or of
working AO. No local evidence currently establishes usable CMR04 depth for
post-processing, valid motion vectors, or true GI.
