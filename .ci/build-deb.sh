#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

: "${DEB_VERSION:?DEB_VERSION must be set (run .ci/set-version.sh first)}"

# apt installs an old rustc (edition 2024 needs the pinned toolchain), so the
# rustup proxies must come before /usr/bin. debian/rules calls `just`, which
# calls `cargo`, so the PATH change also reaches dpkg-buildpackage.
export PATH="$HOME/.cargo/bin:$PATH"

ARTIFACT_DIR="${ARTIFACT_DIR:-$ROOT_DIR/artifacts}"
BUILD_ROOT="$(mktemp -d)"

cleanup() {
  rm -rf "$BUILD_ROOT"
}
trap cleanup EXIT

mkdir -p "$ARTIFACT_DIR"
cp -a "$ROOT_DIR/." "$BUILD_ROOT/cosmic-comp"

cd "$BUILD_ROOT/cosmic-comp"

cargo --version
rustc --version

export DEBFULLNAME="${DEBFULLNAME:-COSMIC Comp CI}"
export DEBEMAIL="${DEBEMAIL:-ci@cosmic-comp.local}"

dch --newversion "$DEB_VERSION" \
  --distribution stable \
  --force-distribution \
  --force-bad-version \
  "Automated CI build from ${SOURCE_SHA:-${GITHUB_SHA:-local}}" < /dev/null

dpkg-buildpackage -us -uc -b

shopt -s nullglob
debs=( "$BUILD_ROOT"/*.deb )
if (( ${#debs[@]} == 0 )); then
  echo "dpkg-buildpackage produced no .deb" >&2
  exit 1
fi

for deb in "${debs[@]}"; do
  cp -v "$deb" "$ARTIFACT_DIR/"
done

ls -la "$ARTIFACT_DIR"
