# rust-poweredup — Copilot instructions

This repository is a Rust port of the `node-poweredup` TypeScript library.
The TypeScript source lives in `node-poweredup/` (a git submodule) and is **read-only reference material** — never modify it.
All Rust code lives in `poweredup/` (at the repo root).

## Non-negotiable Rust conventions

- **Edition 2024** — `edition = "2024"` in every `Cargo.toml`.
- **Async** — `tokio` runtime throughout; no blocking calls on the async executor.
- **Typestate pattern** for the hub connection lifecycle (`Hub<Disconnected>`, `Hub<Connected>`, `Hub<Ready>`).
- **RAII** — connections and subscriptions are released in `Drop`; never leave cleanup to the caller.
- **Enums for everything** — every integer constant from the TS source becomes a `#[repr(u8)] enum` with `TryFrom<u8>`; no raw `u8` constants in logic code.
- **parse-not-validate** — parse raw `&[u8]` into typed enums and structs at the BLE boundary (inside the codec layer); never pass raw bytes through business logic.
- **`tracing` only** for diagnostics — no `println!`, `eprintln!`, `dbg!`, or `log` crate calls.
- **`cargo test`** for all tests — do not run manual terminal commands to execute tests.
- **No unsafe** unless wrapping a C FFI boundary; always document the invariants in a `// SAFETY:` comment.
- **Functional over `mut`** — prefer iterator pipelines (`map`, `filter`, `collect`, `from_fn`, `chunks_exact`) over imperative `mut` accumulator + loop patterns. Only use `mut` bindings when in-place mutation is genuinely clearer (e.g. `BytesMut` builders, `&mut self` state machines).
- **`#![deny(clippy::pedantic)]`** in `lib.rs`; CI must be warning-free. Run `cargo clippy` before committing.
- **`cargo fmt`** — all code must be formatted with `rustfmt` default settings; run `cargo fmt` before committing.
- **No catch-all match arms** — always enumerate every variant explicitly. Only use `_` or `..` when the match target is genuinely open-ended (e.g. a `u8` discriminant, a non-exhaustive external type). Never use `_ =>` to paper over missing arms in project-owned enums.

## Crate structure

```
/ (repo root)
  Cargo.toml                    (workspace)
  poweredup/
    Cargo.toml
    src/
      lib.rs
      error.rs                  (unified Error via thiserror)
      protocol/
        consts.rs               (all enums from consts.ts)
        message.rs              (LpfMessage parse/encode)
      ble/
        mod.rs                  (BleTransport trait)
        btleplug.rs             (real hardware, Linux/RPi)
        mock.rs                 (test simulator)
      hub/mod.rs                (Hub<S> typestate)
      device/
        mod.rs                  (Device trait, DeviceFactory)
        motor/                  (BasicMotor → TachoMotor → AbsoluteMotor hierarchy)
        sensor/
      scanner.rs
  node-poweredup/               (git submodule — read-only)
```

## BLE transport abstraction

Every module that touches BLE must go through the `BleTransport` trait — never call `btleplug` APIs directly from hub or device code. This allows `MockTransport` to substitute in tests.

## Testing

- All unit and integration tests use `MockTransport` — no hardware required.
- Hardware tests are gated with `#[cfg(feature = "hardware-tests")]` and never run in CI.
- Each module must have a `#[cfg(test)]` section covering at minimum: parse round-trip, command encoding, and (for devices) event emission via `MockTransport`.

## Delta workflow

When the `node-poweredup` submodule pointer is bumped (nightly PR from `main`):

1. Diff the submodule: `git diff HEAD~1 HEAD -- node-poweredup`
2. For each changed TS file use the `port-ts-to-rust` skill to update the Rust equivalent.
3. Run `cargo test -p poweredup` from the repo root; fix until green.

## Markdown

Ensure tables' vertical bars are aligned, padded with spaces.