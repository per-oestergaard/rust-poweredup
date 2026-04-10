# Rust port of node-poweredup — Implementation Plan

## Overview

Port the TypeScript `node-poweredup` library to idiomatic Rust.

- The `node-poweredup/` git submodule is **read-only reference material**.
- All Rust code lives in `rust/poweredup/` (a library crate).
- Upstream TS deltas arrive nightly via the submodule pointer CI action.
- Each delta is reviewed and ported via a PR into `main`.

## Core Rust idioms (non-negotiable)

See [`.github/copilot-instructions.md`](.github/copilot-instructions.md) — that file is the single authoritative source for all Rust conventions and is loaded automatically by Copilot on every interaction.

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

1. Nightly CI action bumps `node-poweredup` submodule pointer, opens PR into `main`.
2. Reviewer runs: `git diff HEAD~1 HEAD -- node-poweredup` to see which TS files changed.
3. For each changed TS file, invoke the `port-ts-to-rust` skill.
4. PR merges when `cargo test -p poweredup` is green.

---

## Skills and agents

| File                                          | Purpose                                                            |
| --------------------------------------------- | ------------------------------------------------------------------ |
| `.github/copilot-instructions.md`             | Always-on Rust idiom rules for every AI interaction                |
| `.github/skills/port-ts-to-rust/SKILL.md`     | Repeatable skill: port one TS file → idiomatic Rust                |
| `.github/skills/scaffold-rust-crate/SKILL.md` | One-shot skill: bootstrap Phase 0 workspace/crate                  |
| `.github/skills/cargo-pin/SKILL.md`           | Pin all Cargo.toml deps to exact `=X.Y.Z` versions                 |
| `.github/skills/pin-actions-to-sha/SKILL.md`  | Pin GitHub Actions `uses:` references to SHA hashes                |
| `.github/skills/caveman/SKILL.md`             | Compress memory/instruction files to caveman format (token saving) |
| `.github/agents/device-porter.agent.md`       | Focused agent for device-porting sessions                          |

---

## Example program plan — Battery Box Hub (28738)

The **LEGO Powered UP Bluetooth Hub Battery Box** (set 28738 / item 88009) is the
standard 2-port Powered UP Hub. It contains:

- Port A & B — external motor/sensor ports
- Port 50 — built-in RGB LED (`HubLed`)
- Port 59 — current sensor
- Port 60 — voltage sensor

The example connects to this hub, blinks the LED through a colour sequence, then
starts and stops motors attached to ports A and B.

### Crate layout

```
/
  examples/
    battery_box/
      Cargo.toml    (binary crate: name = "battery_box")
      src/
        main.rs
```

The examples workspace member is feature-gated on `hardware-tests` so it never
pulls in `btleplug` (or compiles at all) during normal `cargo test` CI runs.

### Implementation phases

#### Step 1 — `btleplug` BLE transport (`poweredup/src/ble/btleplug.rs`)

The example requires a real BLE transport. This file is currently a stub (the
`hardware-tests` feature gate references it but it does not yet exist). Implement:

- `BtleplugTransport` struct wrapping a `btleplug::platform::Peripheral`
- `BleTransport` impl: `connect`, `disconnect`, `write`, `subscribe`
- Keep all `btleplug` imports behind `#[cfg(feature = "hardware-tests")]`

#### Step 2 — `btleplug` scanner (`poweredup/src/ble/btleplug.rs` or `scanner.rs`)

Implement `BtleplugScanner::scan() -> impl Stream<Item = AdvertisedHub<BtleplugTransport>>`:

- Use `btleplug::api::Manager::adapters()` → take first adapter
- `adapter.start_scan(ScanFilter { services: [LPF2_SERVICE_UUID] })`
- For each discovered peripheral: read service UUIDs + manufacturer data, call
  `AdvertisedHub::from_advertisement(…)`, yield result
- Stop after the first Powered UP Hub is found (for the example)

#### Step 3 — Example binary (`examples/battery_box/src/main.rs`)

```
cargo run --example battery_box --features hardware-tests
```

Sequence:

1. **Scan** — start BLE scan, wait for the first `HubType::Hub` advertisement
2. **Connect** — `hub.connect().await`
3. **Initialize** — `hub.initialize().await` (reads firmware, battery, RSSI)
4. **Print info** — log hub name, firmware version, battery level
5. **Blink LED** — cycle through Red → Green → Blue → White → Off (500 ms each)
   using `HubLed::encode_set_color` + `Hub::write(LpfMessage::PortOutputCommand(…))`
6. **Start motors** — send `BasicMotorDevice::encode_set_power(75)` on ports A and B
7. **Wait 2 s**
8. **Stop motors** — send `encode_stop()` on both ports
9. **Disconnect** — `hub.disconnect().await`

#### Step 4 — Workspace wiring

- Add `examples/battery_box` as a workspace member in the root `Cargo.toml`
- Gate it: `[workspace] members = [..., "examples/battery_box"]` with a note that
  it requires `--features poweredup/hardware-tests` to build
- Add `tracing-subscriber` as a dev/example dependency for log output

### How to test (hardware required)

**Prerequisites:**

- A Linux machine or Raspberry Pi with a Bluetooth 4.0+ adapter
- The LEGO Powered UP Hub Battery Box (28738) with fresh batteries, power ON
- Rust toolchain installed (`rustup`)
- Two LEGO motors connected to ports A and B (optional — the program handles
  missing devices gracefully)

**Steps:**

```bash
# 1. Clone and enter the repo
git clone --recurse-submodules https://github.com/per-oestergaard/rust-poweredup
cd rust-poweredup

# 2. Build the example (requires btleplug → Linux BLE stack)
cargo build --example battery_box --features poweredup/hardware-tests

# 3. Turn on the hub (press the green button — LED flashes white)

# 4. Run (may need sudo on some Linux distros for BLE access)
cargo run --example battery_box --features poweredup/hardware-tests

# 5. Expected output
# INFO scanning for Powered UP Hub...
# INFO hub found: "Handset" firmware=1.0.00.0000 battery=87%
# INFO blinking LED...
# INFO starting motors on ports A and B...
# INFO stopping motors
# INFO disconnected
```

**Troubleshooting:**

| Symptom                           | Fix                                                                                 |
| --------------------------------- | ----------------------------------------------------------------------------------- |
| `No Bluetooth adapter found`      | Ensure `bluetoothd` is running: `sudo systemctl start bluetooth`                    |
| `Permission denied (os error 13)` | Run with `sudo`, or add your user to the `bluetooth` group                          |
| Hub not found after 30 s          | Hub may have gone to sleep — press button to wake; ensure no other app is connected |
| Motors don't move                 | Check motors are seated fully; hub port LEDs should light when connected            |
