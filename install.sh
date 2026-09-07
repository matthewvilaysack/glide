#!/bin/sh
# Install the latest glide release into ~/.local/bin (or $GLIDE_INSTALL_DIR).
#   curl -fsSL https://raw.githubusercontent.com/matthewvilaysack/glide/main/install.sh | sh
# Set GLIDE_VERSION=0.2.0 to pin. Set GITHUB_TOKEN if the repo is private.
set -eu
repo="matthewvilaysack/glide"
dir="${GLIDE_INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"; arch="$(uname -m)"
case "$os:$arch" in
  Darwin:arm64)   target="aarch64-apple-darwin" ;;
  Darwin:x86_64)  target="x86_64-apple-darwin" ;;
  Linux:x86_64)   target="x86_64-unknown-linux-gnu" ;;
  Linux:aarch64)  target="aarch64-unknown-linux-gnu" ;;
  *) echo "no prebuilt glide for $os/$arch; build from source: cargo install --git https://github.com/$repo glide-cli" >&2; exit 1 ;;
esac

api="https://api.github.com/repos/$repo"
fetch() {
  if [ -n "${GITHUB_TOKEN:-}" ]; then
    curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN" "$@"
  else
    curl -fsSL "$@"
  fi
}
# A private repo's release assets are not served from the plain download URL,
# even with a token; they come through the API asset endpoint instead.
asset() {
  out="$1"; file="$2"
  if [ -n "${GITHUB_TOKEN:-}" ]; then
    id="$(fetch "$api/releases/tags/v$GLIDE_VERSION" | tr -d '\n' | sed -n "s|.*\"url\": *\"[^\"]*/releases/assets/\([0-9]*\)\",[^}]*\"name\": *\"$file\".*|\1|p")"
    [ -n "$id" ] || { echo "no asset named $file on v$GLIDE_VERSION" >&2; exit 1; }
    curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN" -H "Accept: application/octet-stream" -o "$out" "$api/releases/assets/$id"
  else
    curl -fsSL -o "$out" "https://github.com/$repo/releases/download/v$GLIDE_VERSION/$file"
  fi
}

if [ -z "${GLIDE_VERSION:-}" ]; then
  GLIDE_VERSION="$(fetch "$api/releases/latest" | sed -n 's/.*"tag_name": *"v\([^"]*\)".*/\1/p')"
  [ -n "$GLIDE_VERSION" ] || { echo "could not find the latest release" >&2; exit 1; }
fi

name="glide-$GLIDE_VERSION-$target"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
echo "downloading glide $GLIDE_VERSION ($target)"
asset "$tmp/$name.tar.gz" "$name.tar.gz"
asset "$tmp/checksums.txt" "checksums.txt"
want="$(grep " $name.tar.gz\$" "$tmp/checksums.txt" | cut -d' ' -f1)"
have="$(shasum -a 256 "$tmp/$name.tar.gz" | cut -d' ' -f1)"
[ "$want" = "$have" ] || { echo "checksum mismatch for $name.tar.gz" >&2; exit 1; }

tar -xzf "$tmp/$name.tar.gz" -C "$tmp"
mkdir -p "$dir"
install -m 755 "$tmp/$name/glide" "$dir/glide"
echo "installed $dir/glide"
"$dir/glide" --version
case ":$PATH:" in *":$dir:"*) ;; *) echo "add $dir to your PATH" ;; esac
