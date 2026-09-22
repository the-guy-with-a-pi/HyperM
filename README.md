# HyperM

HyperM is an open-source, PM2-style manager for running applications in isolated [Firecracker](https://github.com/firecracker-microvm/firecracker) microVMs. It is written in Rust and keeps the command surface intentionally small.

## Status

This is the first Rust control-plane milestone. It starts one Firecracker VM per app, writes a Firecracker configuration for each VM, persists app metadata, and exposes `run`, `start`, `list`, `logs`, `stop`, and `delete`. A Linux kernel, root filesystem, and guest-side launcher are required because a host command cannot execute directly inside a VM.

## Requirements

- **Supported host OS:** Linux with KVM enabled
- Rust 1.75+
- Firecracker installed and available as `firecracker`
- A compatible Linux kernel and rootfs image

HyperM is designed to run on Linux server distributions such as Ubuntu,
Debian, Fedora, and Alpine when KVM is available. Windows and macOS can be
used for editing or compiling the CLI, but Firecracker microVMs cannot run
there natively.

## Build

```bash
cargo build --release
```

## Usage

The short form is designed to feel like PM2:

```bash
hyperm run app.js
hyperm run app.js --name app-thing --ram 512
```

`run` infers the app name from the filename and uses `/var/lib/hyperm/vmlinux`
and `/var/lib/hyperm/rootfs.ext4` by default. Override those paths with
`--kernel` and `--rootfs` when needed. Extra guest arguments can follow `--`.

For full control, use `start`:

```bash
hyperm start api \
  --kernel /var/lib/hyperm/vmlinux \
  --rootfs /var/lib/hyperm/alpine.ext4 \
  --memory 512 \
  --cpus 1 \
  -- /bin/my-api --port 8080
hyperm list
hyperm logs api
hyperm stop api
hyperm delete api
```

The command is recorded as guest metadata for the guest launcher. The included
rootfs must be prepared to read that metadata or start the intended application
through its init process. HyperM does not claim isolation when Firecracker is
unavailable.

State and per-VM logs are stored under `.hyperm/`; set `HYPERM_HOME` to move them.

## Roadmap

1. Add a guest agent protocol so HyperM can start arbitrary commands inside a prepared guest image.
2. Add resource auto-detection and restart policies.
3. Add KVM checks, VM readiness health checks, and graceful shutdown through the Firecracker API.
4. Add a Unix socket daemon and a small client/dashboard.
5. Publish reproducible guest images and signed release artifacts.

## Development

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

MIT licensed.
