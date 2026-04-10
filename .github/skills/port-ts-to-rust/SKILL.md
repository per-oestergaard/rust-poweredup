---
name: port-ts-to-rust
description: "Port a TypeScript node-poweredup source file to idiomatic Rust. Use when: porting a device class, sensor, motor, hub, or protocol file from TypeScript to Rust; converting protocol constants; translating a hub class; implementing a Rust equivalent of a TS file; processing a submodule delta."
argument-hint: "Path to the TypeScript file relative to node-poweredup/src/, e.g. devices/tachomotor.ts"
---

# Port TypeScript → Rust

Converts one TypeScript source file from `node-poweredup/src/` into the equivalent
idiomatic Rust module under `poweredup/src/`.

## Inputs

- **Argument**: path to the TS file relative to `node-poweredup/src/`
  (e.g. `devices/tachomotor.ts`, `hubs/lpf2hub.ts`, `consts.ts`)

## Procedure

### 1. Read and understand the TS file

Read `node-poweredup/src/<argument>`.
Identify:

- Class hierarchy (`extends`, mixins)
- Public API (methods, properties, events)
- Protocol bytes (Buffer constants, magic numbers)
- Event names and their payload shapes
- Dependencies on other TS files

### 2. Determine Rust target module

Use this mapping:

| TS path                               | Rust module                             |
| ------------------------------------- | --------------------------------------- |
| `consts.ts`                           | `protocol/consts.rs`                    |
| `hubs/lpf2hub.ts` + `hubs/basehub.ts` | `hub/mod.rs` + `hub/lpf2.rs`            |
| `hubs/wedo2smarthub.ts`               | `hub/wedo2.rs`                          |
| `hubs/<specific>.ts`                  | `hub/<snake_case>.rs`                   |
| `devices/basicmotor.ts`               | `device/motor/basic.rs`                 |
| `devices/tachomotor.ts`               | `device/motor/tacho.rs`                 |
| `devices/absolutemotor.ts`            | `device/motor/absolute.rs`              |
| `devices/<motor>.ts`                  | `device/motor/<snake_case>.rs`          |
| `devices/<sensor>.ts`                 | `device/sensor/<snake_case>.rs`         |
| `devices/light.ts`, `hubled.ts` etc.  | `device/light.rs`                       |
| `devices/piezobuzzer.ts`              | `device/buzzer.rs`                      |
| `interfaces.ts`                       | `ble/mod.rs` (the `BleTransport` trait) |
| `utils.ts`                            | `protocol/utils.rs`                     |

### 3. Apply idiomatic Rust translations

| TypeScript pattern                | Rust equivalent                                         |
| --------------------------------- | ------------------------------------------------------- |
| `class Foo extends Bar`           | `struct Foo` implementing the `Bar` trait               |
| `enum Foo { A = 1, B = 2 }`       | `#[repr(u8)] enum Foo { A = 1, B = 2 }` + `TryFrom<u8>` |
| `Buffer.from([0x07, speed, ...])` | `Bytes::copy_from_slice(&[0x07, speed, ...])`           |
| `buf.readInt8(offset)`            | `buf[offset] as i8`                                     |
| `buf.readUInt32LE(offset)`        | `u32::from_le_bytes(buf[offset..offset+4].try_into()?)` |
| `EventEmitter` pattern            | `tokio::sync::broadcast::Sender<Event>`                 |
| `Promise<T>` return               | `impl Future<Output = Result<T>>`                       |
| `setTimeout(fn, ms)`              | `tokio::time::sleep(Duration::from_millis(ms))`         |
| `debug('...')` call               | `tracing::debug!(...)`                                  |
| Raw `number` constants            | Named variants in a `#[repr(u8)] enum`                  |
| `async function`                  | `async fn` with `#[tokio::test]` in tests               |

### 4. Write the Rust file

- Place the new file at `poweredup/src/<module>/<name>.rs`.
- `pub use` the new types from the parent `mod.rs`.
- Follow all rules in `.github/copilot-instructions.md`.
- Do **not** add docstrings to unchanged code; only add comments where logic is non-obvious.

### 5. Add unit tests in the same file

Every ported file must include a `#[cfg(test)]` block covering:

- **Parse round-trip**: construct a raw byte slice matching the TS `Buffer.from([...])` literal,
  parse it, assert the typed value, re-encode, assert bytes match.
- **Command encoding**: call the Rust command method, capture bytes from `MockTransport`,
  assert bytes equal the TS `Buffer.from([...])` literal.
- **Event emission** (devices): feed raw sensor bytes into `receive()`,
  assert the emitted `Event` variant and payload.

### 6. Validate

Run `cargo test -p poweredup` (or instruct the user to do so).
Fix any compile errors or test failures before reporting done.

## Notes

- Never modify files under `node-poweredup/`.
- If the TS file depends on another TS file not yet ported, port that dependency first
  or create a stub with `todo!()` and a `// TODO: port <file>` comment.
- BLE UUID string literals (`"00001623-..."`) become `const` `uuid::Uuid` values in `protocol/consts.rs`.
- `WeDo 2.0` and `LPF2` are **separate protocol paths** — do not mix their codec logic.
