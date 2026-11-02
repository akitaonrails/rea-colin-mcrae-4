#!/bin/sh
# cmr4-play's session configuration on the shared library: stop, the lock,
# the settings window and the commands. Only harmless sleeps run.
. "$(dirname "$(readlink -f "$0")")/helpers"
REA_GPU=
OTHER_PID=
trap '[ -n "$OTHER_PID" ] && kill "$OTHER_PID" 2>/dev/null; rm -rf "$T"' EXIT

# CMR4 never wrote a pid-only lock: such a file with a live pid is ignored
# and never signalled (REA_LEGACY_LOCK=off).
sleep 30 & OTHER_PID=$!
echo "$OTHER_PID" > "$REA_LOCK"
if rea_running; then fail "a pid-only lock counted as running"; else ok "a pid-only lock is ignored"; fi
rea_stop
if kill -0 "$OTHER_PID" 2>/dev/null; then ok "stop never signals a pid-only lock's pid"; else fail "pid-only lock's pid was signalled"; fi
kill "$OTHER_PID" 2>/dev/null; wait "$OTHER_PID" 2>/dev/null; OTHER_PID=

# stop ends the owned session group and then the private prefix's wineserver.
setsid sleep 30 & OWNED=$!
sleep 0.2
stats=$(rea_proc_stat "$OWNED") && printf '%s %s\n' "$OWNED" "${stats#* }" > "$REA_LOCK"
rm -f "$T/wineserver"
rea_stop
wait "$OWNED" 2>/dev/null
if kill -0 "$OWNED" 2>/dev/null; then fail "owned session survived stop"; kill "$OWNED"; else ok "stop signals the owned session group"; fi
if [ ! -f "$REA_LOCK" ]; then ok "stop removes the lock"; else fail "stop left the lock"; fi
contains "stop kills only the private prefix's wineserver" "$(cat "$T/wineserver")" \
  "env WINEPREFIX=$CMR4_PREFIX wineserver -k"

# Spaces and an apostrophe stay in one positional argument, never shell source.
mkdir -p "$T/launcher it's here"
REA_EXE=$T/"launcher it's here"/CMR4Launcher.exe
: > "$REA_EXE"
printf 'resolution = 1920x1080\n' > "$CFGDIR/launcher.cfg"
rea_start > /dev/null
contains "apostrophe+space launcher is one argument" "$(cat "$T/args")" "<$REA_EXE>
<--no-window>"
case "$(cat "$REA_STATE/inner.sh")" in
  *"$REA_EXE"*) fail "launcher path interpolated into shell source" ;;
  *) ok "launcher path absent from generated source" ;;
esac
REA_EXE=$T/CMR4Launcher.exe

# The settings window: Play (0) and unavailable window (4) continue to the
# game; closing it (3) or any other failure does not.
for case in 0:0 4:0 3:1 1:1; do
  echo "${case%:*}" > "$T/rc"
  if rea_settings > /dev/null; then got=0; else got=1; fi
  if [ "$got" = "${case#*:}" ]; then ok "settings exit ${case%:*} -> $got"; else fail "settings exit ${case%:*} gave $got"; fi
done
contains "settings window uses the private prefix and helper flags" "$(cat "$T/settings")" \
  "WINEPREFIX=$CMR4_PREFIX wine $T/CMR4Launcher.exe --settings --linux"

# The commands players use, in a fresh helper run.
play() { sh "$TESTS/../cmr4-play" "$@"; }
contains "status says idle" "$(play status)" "idle"
contains "stop says stopped" "$(play stop)" "stopped"
play bogus > /dev/null 2>&1
if [ "$?" = 2 ]; then ok "an unknown command is a usage error"; else fail "unknown command exit"; fi
# The player's CMR4_* names reach the library; inherited REA_* never do.
REA_PREFIX=$T/elsewhere REA_FUNCTIONS_ONLY=1 REA_PLAY_HERE=$TESTS/.. sh -c \
  '. "$1"; echo "$REA_PREFIX $REA_EXE $REA_RUN $REA_STATE"' sh "$TESTS/../cmr4-play" > "$T/mapped"
contains "CMR4_PREFIX, CMR4_LAUNCHER and CMR4_RUN map" "$(cat "$T/mapped")" \
  "$CMR4_PREFIX $T/CMR4Launcher.exe $T/fake-run $HOME/.config/cmr4-launcher"
finish
