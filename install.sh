#!/bin/sh
set -eu

REPO="saiden-dev/hu"
INSTALL_DIR="${HU_INSTALL_DIR:-/usr/local/bin}"

main() {
  need_cmd curl
  need_cmd uname

  os=$(detect_os)
  arch=$(detect_arch)
  asset=$(asset_name "$os" "$arch")

  if [ -z "$asset" ]; then
    err "unsupported platform: ${os}/${arch}"
  fi

  version=$(latest_version)
  url="https://github.com/${REPO}/releases/download/${version}/${asset}"

  printf "Installing hu %s (%s/%s)\n" "$version" "$os" "$arch"
  printf "  from: %s\n" "$url"
  printf "  to:   %s/hu\n" "$INSTALL_DIR"

  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT

  curl -fsSL "$url" -o "${tmp}/hu"
  chmod +x "${tmp}/hu"

  if [ -w "$INSTALL_DIR" ]; then
    mv "${tmp}/hu" "${INSTALL_DIR}/hu"
  else
    printf "\nElevated permissions required to install to %s\n" "$INSTALL_DIR"
    sudo mv "${tmp}/hu" "${INSTALL_DIR}/hu"
  fi

  printf "\nhu %s installed successfully.\n" "$version"
  printf "Run 'hu --help' to get started.\n"
}

detect_os() {
  case "$(uname -s)" in
    Linux*)  echo "linux" ;;
    Darwin*) echo "macos" ;;
    *)       err "unsupported OS: $(uname -s)" ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)  echo "amd64" ;;
    aarch64|arm64) echo "arm64" ;;
    *)             err "unsupported architecture: $(uname -m)" ;;
  esac
}

asset_name() {
  case "${1}-${2}" in
    macos-arm64)  echo "hu-macos-arm64" ;;
    linux-amd64)  echo "hu-linux-amd64" ;;
    linux-arm64)  echo "hu-linux-arm64" ;;
    *)            echo "" ;;
  esac
}

latest_version() {
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | head -1 \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
}

need_cmd() {
  if ! command -v "$1" > /dev/null 2>&1; then
    err "required command not found: $1"
  fi
}

err() {
  printf "error: %s\n" "$1" >&2
  exit 1
}

main
