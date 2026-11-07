#!/usr/bin/env bash
# Set up the DOS drive for dbcap.py in target/dosbox/dos: QBASIC.EXE from Microsoft's
# OLDDOS.EXE (the MS-DOS 6.22 supplemental disk, via the Internet Archive), GORILLAS.BAS,
# GORSEED.BAS (RANDOMIZE 42 instead of RANDOMIZE (TIMER), for GORILLAS_SEED=42) and the
# font dump programs.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$here/../.."
dl="$root/target/dosbox/download"
dos="$root/target/dosbox/dos"
mkdir -p "$dl" "$dos"
if [ ! -f "$dos/QBASIC.EXE" ]; then
  curl -sSL -o "$dl/OLDDOS.EXE" \
    "https://web.archive.org/web/2004id_/http://download.microsoft.com/download/win95upg/tool_s/1.0/w95/en-us/olddos.exe"
  echo "d06ed2a259d854cf7a7f96a1e1ec1951e6399aa3b9dc613035414ff5b7709fd5  $dl/OLDDOS.EXE" | sha256sum -c -
  unzip -o -q -d "$dl/olddos" "$dl/OLDDOS.EXE" QBASIC.EXE
  cp "$dl/olddos/QBASIC.EXE" "$dos/"
fi
cp "$root/GORILLAS.BAS" "$dos/"
sed 's/RANDOMIZE (TIMER)/RANDOMIZE (42)/' "$root/GORILLAS.BAS" > "$dos/GORSEED.BAS"
for f in FONT14.BAS FONT16.BAS; do perl -pe 's/\r?\n/\r\n/' "$here/$f" > "$dos/$f"; done
echo "DOS drive ready: $dos"
