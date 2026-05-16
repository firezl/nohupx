#!/usr/bin/env sh
set -eu

repo="${NOHUPX_REPO:-firezl/nohupx}"
version="${NOHUPX_VERSION:-latest}"
install_dir="${NOHUPX_INSTALL_DIR:-$HOME/.local/bin}"
bin_name="nohupx"

info() {
  printf '%s\n' "$*"
}

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

need_cmd curl
need_cmd tar
need_cmd uname
need_cmd mktemp

os="$(uname -s)"
arch="$(uname -m)"

if [ "$os" != "Linux" ]; then
  fail "this installer currently supports Linux only; detected: $os"
fi

case "$arch" in
  x86_64 | amd64)
    target="x86_64-unknown-linux-gnu"
    ;;
  *)
    fail "unsupported Linux architecture: $arch"
    ;;
esac

if [ "$version" = "latest" ]; then
  api_url="https://api.github.com/repos/$repo/releases/latest"
  tag="$(
    curl -fsSL "$api_url" |
      sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' |
      head -n 1
  )"
  [ -n "$tag" ] || fail "could not determine latest release from $api_url"
else
  case "$version" in
    v*) tag="$version" ;;
    *) tag="v$version" ;;
  esac
fi

plain_version="${tag#v}"
asset="nohupx-$plain_version-$target.tar.gz"
base_url="https://github.com/$repo/releases/download/$tag"
archive_url="$base_url/$asset"
checksum_url="$archive_url.sha256"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT INT TERM

info "Installing nohupx $tag for $target"
info "Downloading $archive_url"
curl -fL "$archive_url" -o "$tmp_dir/$asset"

info "Downloading checksum"
curl -fL "$checksum_url" -o "$tmp_dir/$asset.sha256"

if command -v sha256sum >/dev/null 2>&1; then
  (
    cd "$tmp_dir"
    sha256sum -c "$asset.sha256"
  )
elif command -v shasum >/dev/null 2>&1; then
  expected="$(awk '{print $1}' "$tmp_dir/$asset.sha256")"
  actual="$(shasum -a 256 "$tmp_dir/$asset" | awk '{print $1}')"
  [ "$expected" = "$actual" ] || fail "checksum mismatch"
else
  fail "required command not found: sha256sum or shasum"
fi

tar -xzf "$tmp_dir/$asset" -C "$tmp_dir"
package_dir="$tmp_dir/nohupx-$plain_version-$target"
[ -x "$package_dir/$bin_name" ] || fail "archive did not contain executable $bin_name"

mkdir -p "$install_dir"
cp "$package_dir/$bin_name" "$install_dir/$bin_name"
chmod +x "$install_dir/$bin_name"

info "Installed: $install_dir/$bin_name"

case ":$PATH:" in
  *":$install_dir:"*) ;;
  *)
    info ""
    info "Add this to your shell profile if nohupx is not found:"
    info "  export PATH=\"$install_dir:\$PATH\""
    ;;
esac

info ""
"$install_dir/$bin_name" --version
info "Done."
