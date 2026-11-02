#!/bin/sh
# Tests CMR04 gamescope resolution/filter and direct-launch policy with fakes.
set -u
unset GAMESCOPE_WAYLAND_DISPLAY DRI_PRIME __NV_PRIME_RENDER_OFFLOAD
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export T HOME=$T CMR4_PREFIX=$T/prefix CMR4_WAIT=1 CMR4_SYSFS=$T/nosys
CFGDIR=$CMR4_PREFIX/drive_c/users/me/AppData/Local/CMR4Launcher
mkdir -p "$CFGDIR"; : > "$T/CMR4Launcher.exe"
cat > "$T/fake-run" <<'FAKE'
#!/bin/sh
case "$*" in
  *"command -v gamescope"*) [ ! -e "$T/no-gamescope" ]; exit ;;
esac
echo "$*" > "$T/ran"
printf '<%s>\n' "$@" > "$T/args"
FAKE
chmod +x "$T/fake-run"
CMR4_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../cmr4-play"
RUN=$T/fake-run LAUNCHER=$T/CMR4Launcher.exe GPU=
fails=0
check() { # name expected-substring
  case "$(cat "$T/ran")" in
    *"$2"*) echo "ok   $1" ;;
    *) echo "FAIL $1: $(cat "$T/ran")"; fails=$((fails + 1)) ;;
  esac
}

printf 'resolution = 3840x2160\n' > "$CFGDIR/launcher.cfg"
CFG=$CFGDIR/launcher.cfg
start > /dev/null
check "4K nested and output, fullscreen fit, NIS" "gamescope -w 3840 -h 2160 -W 3840 -H 2160 -f -S fit -F nis --"
printf 'resolution = 1920x1080\n' > "$CFGDIR/launcher.cfg"
start > /dev/null
check "configured 1080p nested and output" "gamescope -w 1920 -h 1080 -W 1920 -H 1080 -f -S fit -F nis --"
printf 'resolution = original\n' > "$CFGDIR/launcher.cfg"
start > /dev/null
check "original uses a standard original mode" "gamescope -w 1024 -h 768 -W 1024 -H 768"
printf '  resolution = 2560X1440 \r\n' > "$CFGDIR/launcher.cfg"
start > /dev/null
check "launcher spellings: uppercase X, spaces, CRLF" "gamescope -w 2560 -h 1440 -W 2560 -H 1440"
printf 'resolution = 1280x800\n# resolution = 640x480\n' > "$CFGDIR/launcher.cfg"
start > /dev/null
check "commented resolution is ignored" "gamescope -w 1280 -h 800 -W 1280 -H 800"
for bad in huge 4kx x1080 1920x1080x2; do
  printf 'resolution = %s\n' "$bad" > "$CFGDIR/launcher.cfg"
  start > /dev/null
  check "invalid resolution $bad falls back to original mode" "gamescope -w 1024 -h 768 -W 1024 -H 768"
done
printf 'game = C:\\CMR4\nresolution =\n#resolution = 3840x2160\n' > "$CFGDIR/launcher.cfg"
start > /dev/null
check "unset resolution (the template's empty key) uses original mode" "gamescope -w 1024 -h 768 -W 1024 -H 768"
rm "$CFGDIR/launcher.cfg"
start > /dev/null
check "missing launcher.cfg uses original mode" "gamescope -w 1024 -h 768 -W 1024 -H 768"
printf 'resolution = original\n' > "$CFGDIR/launcher.cfg"
check "private Wine prefix is passed" "WINEPREFIX=$CMR4_PREFIX"
check "SDL background events default on" "SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS=1"
check "SDL joystick background events default on" "SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1"
SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS=0 SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=0 start > /dev/null
check "both existing SDL background choices survive" "SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS=0 SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=0"
unset SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS
if [ ! -f "$LOCK" ]; then echo "ok   synchronous natural exit removes lock"; else echo "FAIL natural lock"; fails=$((fails + 1)); fi

GAMESCOPE_WAYLAND_DISPLAY=fake start > /dev/null
check "already in gamescope runs directly" "env WINEPREFIX=$CMR4_PREFIX SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS=1 SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1 sh $HOME/.config/cmr4-launcher/inner.sh"
unset GAMESCOPE_WAYLAND_DISPLAY
: > "$T/no-gamescope"
start > /dev/null
check "unavailable gamescope runs directly" "env WINEPREFIX=$CMR4_PREFIX SDL_GAMECONTROLLER_ALLOW_BACKGROUND_EVENTS=1 SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1 sh $HOME/.config/cmr4-launcher/inner.sh"
[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
