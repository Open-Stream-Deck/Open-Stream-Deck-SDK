#!/bin/sh
set -eu

repo="Open-Stream-Deck/Open-Stream-Deck-SDK"
version="${OPENSD_VERSION:-latest}"
dir="${OPENSD_INSTALL_DIR:-$HOME/.opensd/bin}"

fail() {
    echo "opensd install: $1" >&2
    exit 1
}

case "$(uname -s)" in
    Linux) system="unknown-linux-musl" ;;
    Darwin) system="apple-darwin" ;;
    *) fail "unsupported system $(uname -s); use install.ps1 on Windows" ;;
esac

case "$(uname -m)" in
    x86_64 | amd64) arch="x86_64" ;;
    arm64 | aarch64) arch="aarch64" ;;
    *) fail "unsupported architecture $(uname -m)" ;;
esac

target="$arch-$system"
archive="opensd-$target.tar.gz"
if [ "$version" = "latest" ]; then
    base="https://github.com/$repo/releases/latest/download"
else
    base="https://github.com/$repo/releases/download/v${version#v}"
fi

fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$1" -O "$2"
    else
        fail "curl or wget is required"
    fi
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "Downloading $archive"
fetch "$base/$archive" "$work/$archive"
fetch "$base/SHA256SUMS" "$work/SHA256SUMS"

expected="$(grep " $archive\$" "$work/SHA256SUMS" | cut -d ' ' -f1)"
[ -n "$expected" ] || fail "$archive is not listed in SHA256SUMS"
if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$work/$archive" | cut -d ' ' -f1)"
else
    actual="$(shasum -a 256 "$work/$archive" | cut -d ' ' -f1)"
fi
[ "$expected" = "$actual" ] || fail "checksum mismatch for $archive"

tar -xzf "$work/$archive" -C "$work"
mkdir -p "$dir"
install -m 755 "$work/opensd" "$dir/opensd"

echo "Installed $("$dir/opensd" --version) to $dir/opensd"
case ":$PATH:" in
    *":$dir:"*) ;;
    *) echo "Add $dir to PATH, for example: echo 'export PATH=\"$dir:\$PATH\"' >> ~/.profile" ;;
esac
