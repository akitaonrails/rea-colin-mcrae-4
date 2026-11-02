#!/bin/sh
# Tests CMR04's gamescope size/filter and direct-launch policy with fakes.
. "$(dirname "$(readlink -f "${CMR4_TEST:-$0}")")/helpers"
REA_GPU=
CFG=$CFGDIR/launcher.cfg
run() { rea_start > /dev/null; cat "$T/ran"; }

printf 'resolution = 3840x2160\n' > "$CFG"
contains "4K nested and output, fullscreen fit, NIS" "$(run)" \
  "gamescope -W 3840 -H 2160 -w 3840 -h 2160 -f -S fit -F nis --"
printf 'resolution = 1920x1080\nscaler = fsr\n' > "$CFG"
contains "configured 1080p; the filter is always NIS" "$(run)" \
  "gamescope -W 1920 -H 1080 -w 1920 -h 1080 -f -S fit -F nis --"
printf 'resolution = original\n' > "$CFG"
contains "original uses a standard original mode" "$(run)" "gamescope -W 1024 -H 768 -w 1024 -h 768"
printf '  resolution = 2560X1440 \r\n' > "$CFG"
contains "launcher spellings: uppercase X, spaces, CRLF" "$(run)" "gamescope -W 2560 -H 1440 -w 2560 -h 1440"
printf 'resolution = 1280x800\n# resolution = 640x480\n' > "$CFG"
contains "commented resolution is ignored" "$(run)" "gamescope -W 1280 -H 800 -w 1280 -h 800"
for bad in huge 4kx x1080 1920x1080x2; do
  printf 'resolution = %s\n' "$bad" > "$CFG"
  contains "invalid resolution $bad falls back to original mode" "$(run)" "gamescope -W 1024 -H 768 -w 1024 -h 768"
done
printf 'game = C:\\CMR4\nresolution =\n#resolution = 3840x2160\n' > "$CFG"
contains "unset resolution (the template's empty key) uses original mode" "$(run)" "gamescope -W 1024 -H 768"
rm "$CFG"
contains "missing launcher.cfg uses original mode" "$(run)" "gamescope -W 1024 -H 768"
ran=$(run)
contains "private Wine prefix is passed" "$ran" "WINEPREFIX=$CMR4_PREFIX"
contains "SDL background events default on" "$ran" "SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS=1"
contains "SDL joystick background events default on" "$ran" "SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1"
contains "the launcher runs without its window" "$ran" "$T/CMR4Launcher.exe --no-window"
ran=$(SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS=0 SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=0 run)
contains "existing SDL controller choice survives" "$ran" "SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS=0"
contains "existing SDL joystick choice survives" "$ran" "SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=0"
if [ ! -f "$REA_LOCK" ]; then ok "synchronous natural exit removes lock"; else fail "natural lock"; fi

direct="env WINEPREFIX=$CMR4_PREFIX SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS=1 SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1"
ran=$(GAMESCOPE_WAYLAND_DISPLAY=fake run)
contains "already in gamescope runs directly" "$ran" "$direct"
contains "already in gamescope runs the inner script" "$ran" "sh $HOME/.config/cmr4-launcher/inner.sh"
: > "$T/no-gamescope"
contains "unavailable gamescope runs directly" "$(run)" "$direct"
finish
