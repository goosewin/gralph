#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

PROJECT="$ROOT_DIR/src/Gralph/Gralph.csproj"
OUT_ROOT="$ROOT_DIR/dist/aot"
CONFIGURATION="Release"

ALL_RIDS=(
  "osx-arm64"
  "osx-x64"
  "linux-x64"
  "linux-arm64"
  "win-x64"
)

usage() {
  cat <<'EOF'
Usage: scripts/publish-aot.sh [--rid <RID>] [--all] [--configuration <CONFIG>]

Build Native AOT executables for gralph.

Options:
  --rid <RID>            Build a specific runtime identifier (RID)
  --all                  Build the full RID matrix
  --configuration <CFG>  Build configuration (default: Release)
  -h, --help             Show this help

Examples:
  scripts/publish-aot.sh --rid osx-arm64
  scripts/publish-aot.sh --all
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

RIDS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --rid)
      RIDS+=("$2")
      shift 2
      ;;
    --all)
      RIDS=("${ALL_RIDS[@]}")
      shift
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

if [[ ${#RIDS[@]} -eq 0 ]]; then
  rid="$(host_rid)"
  if [[ -z "$rid" ]]; then
    echo "Error: Unable to determine host RID. Use --rid or --all." >&2
    exit 1
  fi
  RIDS=("$rid")
fi

mkdir -p "$OUT_ROOT"

for rid in "${RIDS[@]}"; do
  out_dir="$OUT_ROOT/$rid"
  mkdir -p "$out_dir"

  dotnet publish "$PROJECT" \
    -c "$CONFIGURATION" \
    -r "$rid" \
    -o "$out_dir" \
    -p:PublishAot=true \
    -p:SelfContained=true \
    -p:PublishSingleFile=true \
    -p:StripSymbols=true \
    -p:InvariantGlobalization=true \
    -p:AssemblyName=gralph

  if [[ "$rid" == win-* ]]; then
    bin_path="$out_dir/gralph.exe"
  else
    bin_path="$out_dir/gralph"
  fi

  if [[ -f "$bin_path" ]]; then
    chmod +x "$bin_path" 2>/dev/null || true
  fi

  echo "Built $rid -> $bin_path"
done
