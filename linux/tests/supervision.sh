#!/bin/sh
# Focused owned-session tests; only harmless sleep and fake launchers run.
set -u
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d)
export T HOME=$T CMR4_PREFIX=$T/prefix CMR4_SYSFS=$T/nosys
STALE_PID= STALE_START= OTHER_PID= OTHER_START=
stop_helper() {
  [ -n "$1" ] || return 0
  stats=$(proc_stat "$1") || return 0
  if [ "${stats#* }" = "$2" ]; then kill "$1" 2>/dev/null || true; fi
  wait "$1" 2>/dev/null || true
}
cleanup() {
  stop_helper "$STALE_PID" "$STALE_START"
  stop_helper "$OTHER_PID" "$OTHER_START"
  rm -rf "$T"
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$CMR4_PREFIX/drive_c/users/me/AppData/Local/CMR4Launcher"
CMR4_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../cmr4-play"
fails=0
ok() { echo "ok   $1"; }
fail() { echo "FAIL $1"; fails=$((fails + 1)); }

# Wine commands are recorded, never run.
cat > "$T/wine-run" <<'FAKE'
#!/bin/sh
echo "$*" >> "$T/wine-ran"
FAKE
chmod +x "$T/wine-run"
RUN=$T/wine-run

# A live pid with the wrong start identity models a reused/stale lock.
sleep 30 & STALE_PID=$!
stats=$(proc_stat "$STALE_PID"); STALE_START=${stats#* }
printf '%s %s\n' "$STALE_PID" "$((STALE_START + 1))" > "$LOCK"
if running; then fail "stale identity reported running"; else ok "stale/reused pid identity refused"; fi
stop_session > /dev/null
if kill -0 "$STALE_PID" 2>/dev/null; then ok "stop did not signal stale/reused pid"; else fail "stale helper was signaled"; fi
stop_helper "$STALE_PID" "$STALE_START"; STALE_PID= STALE_START=

# An unrelated process with the game's name does not own this launch.
ln -s /bin/sleep "$T/cmr4.exe"
"$T/cmr4.exe" 30 & OTHER_PID=$!
stats=$(proc_stat "$OTHER_PID"); OTHER_START=${stats#* }
rm -f "$LOCK"
if running; then fail "unrelated cmr4.exe blocked launch"; else ok "unrelated cmr4.exe does not block"; fi
stop_session > /dev/null
if kill -0 "$OTHER_PID" 2>/dev/null; then ok "unrelated cmr4.exe was not signaled"; else fail "unrelated helper was signaled"; fi
stop_helper "$OTHER_PID" "$OTHER_START"; OTHER_PID= OTHER_START=

# stop ends the owned session group and then the private prefix's wineserver.
setsid sleep 30 & OWNED=$!
sleep 0.2
stats=$(proc_stat "$OWNED") && printf '%s %s\n' "$OWNED" "${stats#* }" > "$LOCK"
: > "$T/wine-ran"
stop_session > /dev/null
wait "$OWNED" 2>/dev/null
if kill -0 "$OWNED" 2>/dev/null; then fail "owned session survived stop"; kill "$OWNED"; else ok "stop signals the owned session group"; fi
if [ ! -f "$LOCK" ]; then ok "stop removes the lock"; else fail "stop left the lock"; fi
case "$(cat "$T/wine-ran")" in
  "env WINEPREFIX=$CMR4_PREFIX wineserver -k") ok "stop kills only the private prefix's wineserver" ;;
  *) fail "stop wine commands: $(cat "$T/wine-ran")" ;;
esac

cat > "$T/fake-run" <<'FAKE'
#!/bin/sh
case "$*" in *"command -v gamescope"*) exit 0 ;; esac
echo "$*" > "$T/ran"
printf '<%s>\n' "$@" > "$T/args"
FAKE
chmod +x "$T/fake-run"
RUN=$T/fake-run GPU=
CFG=$CMR4_PREFIX/drive_c/users/me/AppData/Local/CMR4Launcher/launcher.cfg
printf 'resolution = 1920x1080\n' > "$CFG"
LAUNCHER=$T/CMR4Launcher.exe; : > "$LAUNCHER"
unset CMR4_WAIT || true
start > /dev/null
if [ -f "$LOCK" ]; then fail "natural exit left lock"; else ok "natural supervised exit removes lock"; fi

# Spaces and an apostrophe stay in one positional argument, never shell source.
mkdir -p "$T/launcher it's here"
LAUNCHER=$T/"launcher it's here"/CMR4Launcher.exe
: > "$LAUNCHER"
start > /dev/null
if [ "$(tail -1 "$T/args")" = "<$LAUNCHER>" ]; then ok "apostrophe+space launcher is one argument"; else fail "launcher argument: $(tail -1 "$T/args")"; fi
case "$(cat "$STATE/inner.sh")" in
  *'wine "$1" --no-window'*) ok "inner uses quoted positional launcher" ;;
  *) fail "inner does not use wine \"\$1\"" ;;
esac
case "$(cat "$STATE/inner.sh")" in
  *"$LAUNCHER"*) fail "launcher path interpolated into shell source" ;;
  *) ok "launcher path absent from generated source" ;;
esac

# The settings window: Play (0) and unavailable window (4) continue to the
# game; closing it (3) or any other failure does not.
cat > "$T/fake-run" <<'FAKE'
#!/bin/sh
case "$*" in *"--settings --linux"*) echo "$*" > "$T/settings"; exit "$(cat "$T/rc")" ;; esac
exit 0
FAKE
for case in 0:0 4:0 3:1 1:1; do
  echo "${case%:*}" > "$T/rc"
  if settings; then got=0; else got=1; fi
  if [ "$got" = "${case#*:}" ]; then ok "settings exit ${case%:*} -> $got"; else fail "settings exit ${case%:*} gave $got"; fi
done
case "$(cat "$T/settings")" in
  *"WINEPREFIX=$CMR4_PREFIX"*"wine $LAUNCHER --settings --linux") ok "settings window uses the private prefix and helper flags" ;;
  *) fail "settings command: $(cat "$T/settings")" ;;
esac

[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
