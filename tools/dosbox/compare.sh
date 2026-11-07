#!/usr/bin/env bash
# Run the same session in DOSBox (original) and headless (port) and diff the screenshots.
# usage: compare.sh [SESSION]   (default: session.txt; seeded with RANDOMIZE 42 on both sides)
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$here/../.."
session="${1:-$here/session.txt}"
out="$root/target/compare"
rm -rf "$out/original" "$out/port"
"$here/fetch-qbasic.sh" >/dev/null
python3 "$here/dbcap.py" "$out/original" "$session" GORSEED.BAS
cargo build --release --manifest-path "$root/Cargo.toml"
GORILLAS_SEED=42 "$root/target/release/gorillas" capture "$session" "$out/port" --frames
for f in "$out"/original/*.png; do
  n=$(basename "$f" .png)
  [ -f "$out/port/$n.png" ] || continue
  d=$(compare -metric AE -fuzz 8% "$f" "$out/port/$n.png" "$out/port/diff_$n.png" 2>&1 || true)
  echo "$n: $d pixels differ"
done
