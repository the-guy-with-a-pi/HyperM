# Contributing to HyperM

Thanks for helping improve HyperM.

## Development

HyperM is written in Rust. Install the stable Rust toolchain, then run:

```bash
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Firecracker development and integration testing require Linux with KVM. Unit
and CLI work can be developed on Windows or macOS, but those systems cannot
run the Firecracker backend directly.

## Pull requests

- Keep changes focused and explain the behavior being changed.
- Add or update tests for user-visible behavior.
- Update the README when commands or requirements change.
- Do not commit `.hyperm/`, `target/`, VM images, kernels, or secrets.

By contributing, you agree that your contributions are licensed under the MIT
License.
