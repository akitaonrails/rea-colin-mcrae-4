#!/bin/sh
# Downloads only officially pinned compatibility archives and verifies them.
set -eu
HERE=$(dirname "$(readlink -f "$0")")
CACHE=${1:-${CMR4_CACHE:-$HOME/.cache/cmr4-launcher}}
mkdir -p "$CACHE"

fetch() { # url sha256 filename
  if [ ! -f "$CACHE/$3" ]; then
    rm -f "$CACHE/$3.part"
    curl -fL --retry 3 "$1" -o "$CACHE/$3.part"
    mv "$CACHE/$3.part" "$CACHE/$3"
  fi
  if ! echo "$2  $CACHE/$3" | sha256sum -c --quiet; then
    echo "checksum mismatch: $3" >&2
    rm -f "$CACHE/$3"
    exit 1
  fi
}

while IFS='|' read -r name version url digest file; do
  case "$name" in ''|'#'*) continue ;; esac
  fetch "$url" "$digest" "$file"
  echo "verified $name $version: $CACHE/$file"
done < "$HERE/components.pins"
