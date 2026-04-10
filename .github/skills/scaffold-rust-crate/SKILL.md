---
name: scaffold-rust-crate
description: "Bootstrap the Rust workspace and poweredup library crate for this project. Use when: setting up the Rust crate for the first time, initialising the workspace, creating the initial Cargo.toml files, Phase 0 scaffold."
user-invocable: true
disable-model-invocation: false
---

# Scaffold Rust workspace and poweredup crate (Phase 0)

Creates the complete directory structure and starter files for `rust/poweredup/`
with all module stubs, correct dependencies, and a compiling skeleton.

## Procedure

### 1. Create `Cargo.toml` at the **repo root** (workspace)

```toml
[workspace]
resolver = "2"
members = ["poweredup"]
```

### 2. Create `poweredup/Cargo.toml`

```toml
[package]
name = "poweredup"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio       = { version = "1", features = ["full"] }
tracing     = "0.1"
thiserror   = "2"
bytes       = "1"
uuid        = { version = "1", features = ["v4"] }

# Real BLE hardware (Linux / Raspberry Pi)
[target.'cfg(target_os = "linux")'.dependencies]
btleplug    = { version = "0.11", optional = true }

[features]
hardware-tests = ["btleplug"]

[dev-dependencies]
tokio       = { version = "1", features = ["full", "test-util"] }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### 3. Create module stub files

Create these files with their minimal contents:

**`poweredup/src/lib.rs`**

```rust
pub mod error;
pub mod protocol;
pub mod ble;
pub mod hub;
pub mod device;
pub mod scanner;
```

**`poweredup/src/error.rs`**

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("BLE error: {0}")]
    Ble(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

**`poweredup/src/protocol/mod.rs`**

```rust
pub mod consts;
pub mod message;
```

**`poweredup/src/protocol/consts.rs`** — stub (to be filled by `port-ts-to-rust` skill on `consts.ts`)

```rust
// TODO: port node-poweredup/src/consts.ts
```

**`poweredup/src/protocol/message.rs`** — stub

```rust
// TODO: port LpfMessage codec from node-poweredup/src/hubs/lpf2hub.ts
```

**`poweredup/src/ble/mod.rs`**

```rust
use bytes::Bytes;
use uuid::Uuid;
use crate::error::Result;

pub mod mock;

#[cfg(all(target_os = "linux", feature = "hardware-tests"))]
pub mod btleplug;

/// Abstraction over the BLE transport so MockTransport can substitute in tests.
pub trait BleTransport: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn write(&self, characteristic: Uuid, data: Bytes) -> Result<()>;
    async fn subscribe(
        &self,
        characteristic: Uuid,
    ) -> Result<tokio::sync::mpsc::Receiver<Bytes>>;
}
```

**`poweredup/src/ble/mock.rs`** — stub

```rust
// TODO: implement MockTransport (in-process simulator)
```

**`poweredup/src/hub/mod.rs`** — stub

```rust
// TODO: Hub<S: HubState> typestate (Phase 4)
```

**`poweredup/src/device/mod.rs`** — stub

```rust
// TODO: Device trait, DeviceFactory, PortMap (Phase 5)
pub mod motor;
pub mod sensor;
```

**`poweredup/src/device/motor/mod.rs`** — stub

```rust
// TODO: BasicMotor → TachoMotor → AbsoluteMotor hierarchy (Phase 6)
```

**`poweredup/src/device/sensor/mod.rs`** — stub

```rust
// TODO: sensor device types (Phase 7)
```

**`poweredup/src/scanner.rs`** — stub

```rust
// TODO: BLE scanner (Phase 10)
```

### 4. Validate

After writing all files, check `get_errors` on `poweredup/src/`.
The crate must compile (`cargo build -p poweredup`) with zero errors before this skill is done.
Warnings for empty stubs are acceptable.
