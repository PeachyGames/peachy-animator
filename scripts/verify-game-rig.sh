#!/bin/sh
# Verify one shipped game skeleton against the external Spine runtime without
# modifying or committing it. Results and copies stay under .local/artist/.
#
#   scripts/verify-game-rig.sh <short-name> <skeleton.json> [--full]
#
# 1. SHA-256 of the source, recorded and re-checked at the end (must be OK).
# 2. Import to native and export back to Spine JSON.
# 3. check-rig-skins.mjs: per skin, bone activity, world transforms and mesh
#    vertices, ours (CLI) vs runtime.
# 4. check-rig-reexport.mjs: original vs our re-export through the runtime
#    with physics on (lossless round trip check).
# 5. --full: scripts/check-runtime.mjs (adds draw order, colors, regions,
#    images/blend, clipping coverage). Slow on big rigs (hours).
#
# The CLI is copied to a per-run snapshot so rebuilding the workspace during
# a long check cannot swap the binary underneath it. The snapshot is removed
# at the end; never delete it while a check is still running.
set -eu
name=$1
source=$2
full=${3:-}
root=$(cd "$(dirname "$0")/.." && pwd)
runtime=${PEACHY_RUNTIME:-$root/../loadofftemp-release-0.51.0/node_modules/@esotericsoftware/spine-core/dist/index.js}
cargo=${PEACHY_CARGO:-$root/../.local-tools/peachy-animator/cargo}
base=${source%.json}
if [ -f "$base.atlas.txt" ]; then atlas="$base.atlas.txt"; else atlas="$base.atlas"; fi
out="$root/.local/artist"
mkdir -p "$out"
"$cargo" build -j 2 -p peachy-cli >/dev/null 2>&1
cli="$root/.local/peachy-cli-$name"
cp "$root/target/debug/peachy-cli" "$cli"
shasum -a 256 "$source" > "$out/$name.sha256"
rm -f "$out/$name.peachy" "$out/$name.json"
"$cli" import "$source" "$out/$name.peachy"
"$cli" export "$out/$name.peachy" "$out/$name.json" >/dev/null
echo "== skins vs ours"
node "$root/scripts/check-rig-skins.mjs" "$runtime" "$atlas" "$out/$name.json" "$out/$name.peachy" "$cli" 2
echo "== original vs re-export (physics on)"
node "$root/scripts/check-rig-reexport.mjs" "$runtime" "$atlas" "$source" "$out/$name.json" 3
if [ "$full" = "--full" ]; then
  echo "== full runtime check"
  PEACHY_CLI="$cli" node "$root/scripts/check-runtime.mjs" "$runtime" "$out/$name.json" "$atlas" "$out/$name.peachy" > "$out/$name-runtime.txt" 2>&1 \
    && echo "full check passed: $out/$name-runtime.txt" || echo "full check FAILED: $out/$name-runtime.txt"
fi
shasum -a 256 -c "$out/$name.sha256"
rm "$cli"
