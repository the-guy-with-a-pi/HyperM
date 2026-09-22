#!/usr/bin/env bash
set -Eeuo pipefail

FIRECRACKER_VERSION="${FIRECRACKER_VERSION:-1.9.1}"
KERNEL_VERSION="${HYPERM_KERNEL_VERSION:-5.10.225}"
KERNEL_RELEASE="${HYPERM_KERNEL_RELEASE:-1.9}"
ROOTFS_URL="${HYPERM_ROOTFS_URL:-https://s3.amazonaws.com/spec.ccfc.min/firecracker-ci/v1.9/$(uname -m)/ubuntu-22.04.ext4}"
REPOSITORY_URL="${HYPERM_REPOSITORY:-https://github.com/the_guy_with_a_pi/HyperM.git}"
WORK_DIR=""

cleanup() {
  if [[ -n "$WORK_DIR" && -d "$WORK_DIR" ]]; then
    rm -rf "$WORK_DIR"
  fi
}
trap cleanup EXIT

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "HyperM installation requires Linux with KVM." >&2
  exit 1
fi

case "$(uname -m)" in
  x86_64) firecracker_arch="x86_64" ;;
  aarch64|arm64) firecracker_arch="aarch64" ;;
  *)
    echo "This installer supports Linux x86_64 and aarch64 only. Install 64-bit Raspberry Pi OS." >&2
    exit 1
    ;;
esac

if [[ "$EUID" -ne 0 ]]; then
  echo "Run this installer with sudo: sudo ./install.sh" >&2
  exit 1
fi

if [[ -e /dev/kvm ]]; then
  echo "KVM device found."
else
  echo "Warning: /dev/kvm is missing. HyperM will install, but Firecracker cannot start until KVM is available." >&2
fi

install_packages() {
  if command -v apt-get >/dev/null 2>&1; then
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y ca-certificates curl git build-essential cargo rustc
  elif command -v dnf >/dev/null 2>&1; then
    dnf install -y ca-certificates curl git gcc gcc-c++ make cargo rust
  elif command -v apk >/dev/null 2>&1; then
    apk add --no-cache ca-certificates curl git build-base cargo rust
  else
    echo "Unsupported package manager. Install curl, git, a C compiler, Cargo, and Rust manually." >&2
    exit 1
  fi
}

install_packages

if ! command -v curl >/dev/null 2>&1 || ! command -v cargo >/dev/null 2>&1; then
  echo "Required tools were not installed successfully." >&2
  exit 1
fi

firecracker_archive="firecracker-v${FIRECRACKER_VERSION}-${firecracker_arch}.tgz"
firecracker_url="https://github.com/firecracker-microvm/firecracker/releases/download/v${FIRECRACKER_VERSION}/${firecracker_archive}"
WORK_DIR="$(mktemp -d)"

echo "Installing Firecracker ${FIRECRACKER_VERSION}..."
curl --fail --location --retry 3 --output "$WORK_DIR/$firecracker_archive" "$firecracker_url"
tar -xzf "$WORK_DIR/$firecracker_archive" -C "$WORK_DIR"
firecracker_binary="$WORK_DIR/release-v${FIRECRACKER_VERSION}-${firecracker_arch}/firecracker-v${FIRECRACKER_VERSION}-${firecracker_arch}"
if [[ ! -f "$firecracker_binary" ]]; then
  echo "Firecracker archive layout was not recognized. Check the release archive manually." >&2
  exit 1
fi
install -m 0755 "$firecracker_binary" /usr/local/bin/firecracker

install -d -m 0755 /var/lib/hyperm
kernel_url="${HYPERM_KERNEL_URL:-https://s3.amazonaws.com/spec.ccfc.min/firecracker-ci/v${KERNEL_RELEASE}/${firecracker_arch}/vmlinux-${KERNEL_VERSION}}"
echo "Installing Firecracker-compatible ${firecracker_arch} kernel..."
curl --fail --location --retry 3 --output /var/lib/hyperm/vmlinux "$kernel_url"
chmod 0644 /var/lib/hyperm/vmlinux
echo "Installing default Ubuntu 22.04 rootfs..."
curl --fail --location --retry 3 --output /var/lib/hyperm/rootfs.ext4 "$ROOTFS_URL"
chmod 0644 /var/lib/hyperm/rootfs.ext4

if [[ -f Cargo.toml ]]; then
  source_dir="$PWD"
else
  echo "Cloning HyperM..."
  git clone --depth 1 "$REPOSITORY_URL" "$WORK_DIR/HyperM"
  source_dir="$WORK_DIR/HyperM"
fi

echo "Building HyperM..."
cargo install --path "$source_dir" --root /usr/local

installer_user="${SUDO_USER:-}"
if [[ -n "$installer_user" ]] && id "$installer_user" >/dev/null 2>&1 && getent group kvm >/dev/null 2>&1; then
  usermod -aG kvm "$installer_user"
  echo "Added $installer_user to the kvm group. Log out and back in before running HyperM."
fi

firecracker --version
hyperm --help >/dev/null

echo
echo "HyperM installation complete."
echo "A compatible kernel was installed at /var/lib/hyperm/vmlinux."
echo "A default Ubuntu 22.04 rootfs was installed at /var/lib/hyperm/rootfs.ext4."
echo "Then run: hyperm run app.js --name app --ram 512"
