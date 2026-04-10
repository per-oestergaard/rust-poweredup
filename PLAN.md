# Rust port of node-poweredup — Implementation Plan

## Overview

Port the TypeScript `node-poweredup` library to idiomatic Rust.

- The `node-poweredup/` git submodule is **read-only reference material**.
- All Rust code lives in `rust/poweredup/` (a library crate).
- Upstream TS deltas arrive nightly via the submodule pointer CI action.
- Each delta is reviewed and ported via a PR from `main` into `rust-main`.

## Core Rust idioms (non-negotiable)

| Principle          | How applied                                                                                     |
| ------------------ | ----------------------------------------------------------------------------------------------- |
| Edition 2024       | `edition = "2024"` in every `Cargo.toml`                                                        |
| Async              | `tokio` runtime throughout                                                                      |
| Typestate          | `Hub<S: HubState>` — compiler-enforced connection lifecycle                                     |
| RAII               | `Drop` on connected hubs sends HUB_ACTION Disconnect                                            |
| Enums              | Every integer constant from TS becomes a `#[repr(u8)] enum` with `TryFrom<u8>`                  |
| parse-not-validate | Raw `&[u8]` parsed into typed enums **at the BLE boundary** — never passed through logic layers |
| Diagnostics        | `tracing` crate only — no `println!`, no `eprintln!`                                            |
| Testing            | `cargo test` — never manual terminal commands                                                   |

## Target architecture

```
/ (repo root)
  Cargo.toml                         (workspace — members = ["poweredup"])
  poweredup/
    Cargo.toml
    src/
      lib.rs
      error.rs                       (unified Error enum via thiserror)
      protocol/
        consts.rs                    (all enums from consts.ts)  ← Phase 1 DONE
        message.rs                   (LpfMessage typed enum; parse/encode at BLE boundary)
      ble/
        mod.rs                       (BleTransport trait)        ← Phase 2 skeleton done
        btleplug.rs                  (real hardware: Linux / Raspberry Pi)
        mock.rs                      (in-process simulator; drives all unit/integration tests)
      hub/
        mod.rs                       (Hub<S> typestate + RAII)
        lpf2.rs                      (LPF2 message dispatch)
        wedo2.rs                     (WeDo 2.0 separate protocol)
      device/
        mod.rs                       (Device trait; DeviceFactory; PortMap)
        motor/
          basic.rs                   (set_power, brake, stop)
          tacho.rs                   (set_speed, rotate_by_degrees, ramps)
          absolute.rs                (goto_angle, reset_zero)
          *.rs                       (9 concrete motor types)
        sensor/                      (per-sensor-type modules)
        light.rs
        buzzer.rs
      scanner.rs                     (BLE scan → HubType → Hub instance)
  node-poweredup/                    (git submodule — read-only TS reference)
```

**Key dependencies:** `btleplug`, `tokio`, `tracing`, `thiserror`, `bytes`

---

## Phased plan

### Phase 0 — Scaffold _(prerequisite for everything)_

1. `rust/Cargo.toml` workspace
2. `rust/poweredup/Cargo.toml` library crate (edition 2024)
3. `poweredup/src/lib.rs` — module skeleton
4. `poweredup/src/error.rs` — `Error` enum via `thiserror`

**Done when:** `cargo build -p poweredup` compiles with no warnings.

---

### Phase 1 — Protocol constants _(parallel with Phase 0)_

Source: `node-poweredup/src/consts.ts`

- `poweredup/src/protocol/consts.rs`
  - `HubType`, `DeviceType`, `MessageType`, `CommandFeedback` — `#[repr(u8)]` enums with `TryFrom<u8>`
  - BLE UUID constants as `const uuid::Uuid`
  - `EndState`, `BrakingStyle`, `ProfileBit` motor constants

**Done when:** all enums compile and have exhaustive `TryFrom<u8>` round-trip tests.

---

### Phase 2 — BLE abstraction + mock transport _(depends on Phase 0)_

- `BleTransport` trait:
  ```rust
  async fn connect(&mut self) -> Result<()>;
  async fn disconnect(&mut self) -> Result<()>;
  async fn write(&self, characteristic: Uuid, data: Bytes) -> Result<()>;
  async fn subscribe(&self, characteristic: Uuid) -> Result<mpsc::Receiver<Bytes>>;
  ```
- `BtleplugTransport` — real hardware (Linux only, feature-gated)
- `MockTransport` — pre-scripted inbound messages + write capture; used in all tests

**Done when:** `MockTransport` can be connected, written to, and delivers scripted messages.

---

### Phase 3 — LPF2 message codec _(depends on Phases 1–2)_

Source: `node-poweredup/src/hubs/lpf2hub.ts` `_parseMessage()` + each device `receive()`

- `LpfMessage` — typed enum with a variant per `MessageType`
- `fn parse(buf: &[u8]) -> Result<LpfMessage>` — parse-not-validate entry point
- `fn encode(msg: &LpfMessage) -> Bytes` — command serialisation
- `SensorValue` — typed enum for all sensor data variants
- `PortOutputPayload` — typed enum for all motor opcodes

**Done when:** round-trip parse/encode tests pass for every message type using byte slices
from the LEGO Wireless Protocol spec and TS source comments.

---

### Phase 4 — Hub typestate _(depends on Phase 3)_

Source: `node-poweredup/src/hubs/basehub.ts` + `lpf2hub.ts`

- `Hub<Disconnected>` → `Hub<Connected>` → `Hub<Ready>`
- `Drop` on `Hub<Connected>` / `Hub<Ready>` sends HUB_ACTION Disconnect
- `connect(transport)` → subscribes LPF2_ALL, streams messages via Tokio channel
- `initialize()` → requests firmware, battery, MAC, RSSI, button state; spawns dispatch task

**Done when:** mock-transport test connects, initializes, and receives hub property events.

---

### Phase 5 — Device trait + attachment _(depends on Phase 4)_

Source: `node-poweredup/src/devices/device.ts`

- `Device` trait: `fn receive(&mut self, mode: u8, data: &[u8]) -> Result<Event>`
- `DeviceFactory::create(device_type: DeviceType, port: u8) -> Box<dyn Device>`
- `PortMap` per hub type (port name → port id)
- Port attachment/detachment from `HUB_ATTACHED_IO` messages parsed in Phase 3

**Done when:** mock attach/detach events round-trip through hub to device registry.

---

### Phase 6 — Motor hierarchy _(depends on Phase 5; highest priority)_

Source: `basicmotor.ts` → `tachomotor.ts` → `absolutemotor.ts` → concrete motor files

- `BasicMotor` trait — `set_power()`, `brake()`, `stop()`
- `TachoMotor` trait — `set_speed()`, `rotate_by_degrees()`, accel/decel ramp
- `AbsoluteMotor` trait — `goto_angle()`, `reset_zero()`
- 9 concrete types implementing relevant trait(s)
- Command queue + `CommandFeedbackFuture` (Rust equivalent of TS Promise)

**Done when:** motor commands produce correctly-encoded byte sequences (unit tests),
and feedback state machine resolves the future correctly (mock transport integration test).

---

### Phase 7 — Sensor types _(parallel with Phase 6)_

Source: sensor files in `node-poweredup/src/devices/`

Priority order:

1. `ColorDistanceSensor`, `TechnicColorSensor`, `TechnicDistanceSensor`, `TechnicForceSensor`
2. `TiltSensor`, `MoveHubTiltSensor`, `MotionSensor`
3. Built-in hub sensors: `Accelerometer`, `GyroSensor`, `TiltSensor (internal)`
4. `VoltageSensor`, `CurrentSensor`
5. `RemoteControlButton`

---

### Phase 8 — Light and audio devices _(parallel with Phase 7)_

- `HubLed`, `Light`, `Technic3x3ColorLightMatrix`, `PiezoBuzzer`

---

### Phase 9 — Hub subclasses _(depends on Phases 5–8)_

- `StandardHub` (ports A, B, LED, current, voltage)
- `MoveHub`, `TechnicMediumHub`, `TechnicSmallHub`
- `RemoteControl`, `DuploTrainBase`
- Mario family (lowest priority)

---

### Phase 10 — Scanner / entry point _(depends on Phase 9)_

Source: `node-poweredup/src/poweredup-node.ts`

- Scan BLE advertisements → parse manufacturer data → `HubType` → typed `Hub` instance

---

### Phase 11 — WeDo 2.0 _(separate protocol; lowest priority)_

Source: `node-poweredup/src/hubs/wedo2smarthub.ts`

---

## Testing strategy

| Scope           | Transport                | How to run                                                              |
| --------------- | ------------------------ | ----------------------------------------------------------------------- |
| Protocol codec  | none                     | `cargo test -p poweredup`                                               |
| Device logic    | `MockTransport`          | `cargo test -p poweredup`                                               |
| Hub integration | `MockTransport`          | `cargo test -p poweredup`                                               |
| Hardware        | real `BtleplugTransport` | `cargo test -p poweredup --features hardware-tests` (Raspberry Pi only) |

Hardware tests are gated behind `#[cfg(feature = "hardware-tests")]` and never run in CI.

---

## PR delta workflow

1. Nightly CI action bumps `node-poweredup` submodule pointer, opens PR `main` → `rust-main`.
2. Reviewer runs: `git diff HEAD~1 HEAD -- node-poweredup` to see which TS files changed.
3. For each changed TS file, invoke the `port-ts-to-rust` skill.
4. PR merges when `cargo test -p poweredup` is green.

---

## Skills and agents

| File                                          | Purpose                                             |
| --------------------------------------------- | --------------------------------------------------- |
| `.github/copilot-instructions.md`             | Always-on Rust idiom rules for every AI interaction |
| `.github/skills/port-ts-to-rust/SKILL.md`     | Repeatable skill: port one TS file → idiomatic Rust |
| `.github/skills/scaffold-rust-crate/SKILL.md` | One-shot skill: bootstrap Phase 0 workspace/crate   |
| `.github/agents/device-porter.agent.md`       | Focused agent for device-porting sessions           |
