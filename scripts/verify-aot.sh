#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

RID=""
RUNS=15
CONFIGURATION="Release"

usage() {
  cat <<'EOF'
Usage: scripts/verify-aot.sh [--rid <RID>] [--runs <N>] [--configuration <CFG>]

Builds and measures startup time + binary size for a Native AOT build.

Options:
  --rid <RID>            Runtime identifier (defaults to host RID)
  --runs <N>             Number of startup samples (default: 15)
  --configuration <CFG>  Build configuration (default: Release)
  -h, --help             Show this help
EOF
}

host_rid() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin)
      case "$arch" in
        arm64) echo "osx-arm64" ;;
        x86_64) echo "osx-x64" ;;
        *) echo "" ;;
      esac
      ;;
    Linux)
      case "$arch" in
        x86_64) echo "linux-x64" ;;
        aarch64|arm64) echo "linux-arm64" ;;
        *) echo "" ;;
      esac
      ;;
    MINGW*|MSYS*|CYGWIN*)
      case "$arch" in
        x86_64) echo "win-x64" ;;
        *) echo "" ;;
      esac
      ;;
    *)
      echo "" ;;
  esac
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rid)
      RID="$2"
      shift 2
      ;;
    --runs)
      RUNS="$2"
      shift 2
      ;;
    --configuration)
      CONFIGURATION="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "$RID" ]]; then
  RID="$(host_rid)"
  if [[ -z "$RID" ]]; then
    echo "Error: Unable to determine host RID. Use --rid." >&2
    exit 1
  fi
fi

"$SCRIPT_DIR/publish-aot.sh" --rid "$RID" --configuration "$CONFIGURATION"

if [[ "$RID" == win-* ]]; then
  BIN_PATH="$ROOT_DIR/dist/aot/$RID/gralph.exe"
else
  BIN_PATH="$ROOT_DIR/dist/aot/$RID/gralph"
fi

if [[ ! -f "$BIN_PATH" ]]; then
  echo "Error: Expected binary not found at $BIN_PATH" >&2
  exit 1
fi

python3 - "$BIN_PATH" "$RUNS" "$RID" <<'PY'
import os
import statistics
import subprocess
import sys
import time

bin_path = sys.argv[1]
runs = int(sys.argv[2])
rid = sys.argv[3]

times = []
for _ in range(runs):
    start = time.perf_counter()
    subprocess.run([bin_path, "--version"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
    end = time.perf_counter()
    times.append((end - start) * 1000.0)

times.sort()
median = statistics.median(times)
index = int(0.95 * (len(times) - 1))
p95 = times[index]
size_bytes = os.path.getsize(bin_path)

print(f"rid={rid}")
print(f"binary={bin_path}")
print(f"size_bytes={size_bytes}")
print(f"runs={runs}")
print(f"startup_median_ms={median:.1f}")
print(f"startup_p95_ms={p95:.1f}")
PY
