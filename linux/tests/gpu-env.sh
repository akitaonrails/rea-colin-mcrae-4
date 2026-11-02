#!/bin/sh
# Tests cmr4-play's GPU choice (the library's offload rules, not pinned)
# against fake sysfs trees.
. "$(dirname "$(readlink -f "$0")")/helpers"
gpu() { # tree card vendor driver boot_vga [vram]
  d=$T/$1/class/drm/$2/device
  mkdir -p "$d" "$T/$1/drivers/$4" "$T/$1/class/drm/$2-DP-1"
  echo "$3" > "$d/vendor"; echo "$5" > "$d/boot_vga"
  ln -s "$T/$1/drivers/$4" "$d/driver"
  [ -n "${6:-}" ] && echo "$6" > "$d/mem_info_vram_total"
  return 0
}
check() { # name tree expected [NV_LIB]
  if [ -n "${4:-}" ]; then REA_NV_LIB_CHECK=true; else REA_NV_LIB_CHECK=false; fi
  got=$(REA_SYSFS=$T/$2 rea_gpu_env)
  if [ "$got" = "$3" ]; then ok "$1"; else fail "$1: got '$got', want '$3'"; fi
}
NV="__NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia __VK_LAYER_NV_optimus=NVIDIA_only"

gpu laptop card0 0x8086 i915 1; gpu laptop card1 0x10de nvidia 0
check "Intel laptop screen + NVIDIA dGPU" laptop "$NV" yes
check "NVIDIA without its OpenGL library: Mesa rule" laptop "DRI_PRIME=1"
gpu dgpu card0 0x1002 amdgpu 1 8589934592; gpu dgpu card1 0x1002 amdgpu 0 536870912
check "AMD dGPU on the screen + AMD APU" dgpu ""
DRI_PRIME=0 check "the player's DRI_PRIME wins" laptop ""
finish
