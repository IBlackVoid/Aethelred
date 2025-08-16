# Aethelred

Aethelred is a lightweight, **Rust-powered container runtime** designed for developers who want to understand how containers work under the hood while still getting productive tooling out-of-the-box. It follows the *do-one-thing-well* philosophy: provide a small, hackable core capable of spawning and managing isolated Linux processes via namespaces, cgroups and other kernel primitives.

> **Why the name?**  Æthelred was an Anglo-Saxon king whose name means *“noble counsel.”*  Likewise, this project aims to offer clear, approachable guidance for building container tech in Rust.

---

## ✨ Key Features

| Area          | What you get |
|---------------|--------------|
| **Namespaces**| PID, UTS, Mount, IPC, Network and User namespace isolation |
| **cgroups v2**| Fine-grained resource limits for CPU, memory, PIDs, I/O |
| **OCI-style CLI** | Familiar `run`, `ps`, `logs`, `stop` sub-commands |
| **Daemon (aethel-d)** | gRPC server that supervises containers |
| **Image storage** | Simple on-disk image layout + demo Alpine rootfs |
| **Tests** | Unit & E2E test suite (`aethel-tests`) |

---

## 📦 Repository Layout

| Crate                | Purpose |
|----------------------|---------|
| `aethel-cli`         | Human-facing command-line interface |
| `aethel-d`           | Long-running daemon that talks to the kernel |
| `aethel-common`      | Error types, protobuf definitions, shared helpers |
| `aethel-net`         | Networking utilities (bridge setup, veth, etc.) |
| `aethel-run`         | Core runtime responsible for namespaces/cgroups |
| `aethel-storage`     | Local image and layer management |
| `aethel-tests`       | End-to-end integration tests |

---

## 🚀 Quick Start

### Prerequisites

* **Linux** (Kernel ≥ 5.4).  Namespaces & cgroups require root privileges.
* **Rust stable** (via `rustup`).
* `protobuf` compiler (`protoc`) for regenerating gRPC stubs.

### Build Everything

```bash
# Clone
$ git clone https://github.com/IBlackVoid/Aethelred.git
$ cd Aethelred

# Compile all crates in release mode
$ cargo build --release
```

### Run the Daemon

```bash
# Needs root for namespace & cgroup operations
$ sudo ./target/release/aethel-d
```
The daemon listens on a local Unix socket (`/run/aethelred.sock` by default).

### Interact via CLI

```bash
# Launch an Alpine container (demo rootfs included)
$ cargo run -p aethel-cli -- run demo-alpine

# Show running containers
$ cargo run -p aethel-cli -- ps

# Stream logs
$ cargo run -p aethel-cli -- logs -f <container-id>

# Stop a container
$ cargo run -p aethel-cli -- stop <container-id>
```

### Run Tests

```bash
# All unit tests
$ cargo test --all

# End-to-end tests
$ cargo test -p aethel-tests
```

---

## 🛠️ Development

1. **Regenerate protobufs** whenever you change `proto/aethel.proto`:
   ```bash
   $ cargo run -p aethel-common --bin build-proto
   ```
2. **Code formatting & linting**:
   ```bash
   $ cargo fmt --all
   $ cargo clippy --all-targets -- -D warnings
   ```
3. **Hot reload daemon** while hacking on system code:
   ```bash
   $ systemfd --no-pid -s unix:/run/aethelred.sock -- cargo watch -x "run -p aethel-d"
   ```

---

## 🤝 Contributing

Issues and pull requests are very welcome!  Please:

1. Open a discussion/issue first if you plan substantial changes.
2. Ensure `cargo test --all` passes and `cargo fmt` leaves no diff.
3. Keep commit messages concise and conventional (e.g., `feat:`, `fix:`, `docs:`).

---

## 📜 License

This project is licensed under the **MIT License**.  See `LICENSE` for details.

---

Made with ❤️ & `unsafe {}` by [@IBlackVoid](https://github.com/IBlackVoid)