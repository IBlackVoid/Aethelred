# Aethelred

Aethelred is a Linux container runtime project in Rust.

It exists to learn how container runtimes actually work under the hood:
- namespaces,
- rootfs setup,
- daemon/CLI control plane,
- and lifecycle wiring.

## Scope

This is an educational runtime, not a Docker replacement.
It is intentionally small and readable.

## Features

- Process and filesystem isolation (Linux namespaces + `pivot_root` path).
- Rootfs preparation for OCI-style image layers.
- gRPC daemon + CLI.
- Basic lifecycle commands: `run`, `ps`, `stop`, `logs`.

## Requirements

- Linux host.
- Rust toolchain.
- Root privileges.

## Build

```bash
cargo build --release -p aethel-d
```

## Run

```bash
sudo ./target/release/aethel-d
```

## CLI

```bash
cargo run -p aethel-cli -- run --image busybox /bin/sh
cargo run -p aethel-cli -- ps
cargo run -p aethel-cli -- logs --container-id <container-id>
cargo run -p aethel-cli -- stop --container-id <container-id>
```

## Known Limits

- Linux-only by design.
- Hardening is incomplete (security model and ops model are minimal).
- Logging/streaming is basic and can be extended.
