#!/usr/bin/env bash
# Render an AUR package directory from its PKGBUILD.in template.
#
#   packaging/aur/render.sh <voidlink|voidlink-bin> <version> <out-dir> [pkgrel]
#
# Downloads the release artifacts to compute checksums, so the release must
# already be published. Writes PKGBUILD and voidlink.install into <out-dir>;
# generate .SRCINFO there afterwards with `makepkg --printsrcinfo`.
set -euo pipefail

pkg="${1:?package name}"
version="${2:?version, without the leading v}"
out="${3:?output directory}"
pkgrel="${4:-1}"
version="${version#v}"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="https://github.com/CPT-Dawn/VoidLink"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

sha() {
  curl -fsSL --retry 3 "$1" -o "$tmp/artifact"
  sha256sum "$tmp/artifact" | cut -d' ' -f1
}

sed_args=(-e "s|@PKGVER@|$version|g" -e "s|@PKGREL@|$pkgrel|g")
case "$pkg" in
  voidlink)
    sed_args+=(-e "s|@SHA256_SRC@|$(sha "$repo/archive/refs/tags/v$version.tar.gz")|g")
    ;;
  voidlink-bin)
    base="$repo/releases/download/v$version/voidlink-$version"
    sed_args+=(-e "s|@SHA256_X86_64@|$(sha "$base-x86_64-linux.tar.gz")|g")
    sed_args+=(-e "s|@SHA256_AARCH64@|$(sha "$base-aarch64-linux.tar.gz")|g")
    ;;
  *)
    echo "unknown package: $pkg" >&2
    exit 1
    ;;
esac

mkdir -p "$out"
sed "${sed_args[@]}" "$here/$pkg/PKGBUILD.in" > "$out/PKGBUILD"
cp "$here/voidlink.install" "$out/voidlink.install"
if grep -q '@[A-Z0-9_]*@' "$out/PKGBUILD"; then
  echo "unrendered placeholder left in $out/PKGBUILD" >&2
  exit 1
fi
echo "Rendered $pkg $version-$pkgrel into $out"
