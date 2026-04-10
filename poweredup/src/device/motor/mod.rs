//! Motor device hierarchy — ported from `basicmotor.ts` / `tachomotor.ts` / `absolutemotor.ts`.
//!
//! Three levels of capability, each adding commands on top of the previous:
//!
//! ```text
//! BasicMotorDevice   — set_power, ramp_power, stop, brake
//!   └─ TachoMotorDevice — set_speed, rotate_by_degrees, set_acceleration_time,
//!                         set_deceleration_time  (+rotation encoder events)
//!        └─ AbsoluteMotorDevice — goto_angle, reset_zero  (+absolute-position events)
//! ```
//!
//! Commands are encoded as raw `PORT_OUTPUT_COMMAND` payloads and returned as
//! [`Bytes`] for the hub layer to transmit.  No BLE calls happen inside this module.

use bytes::Bytes;

use crate::{error::Result, protocol::consts::BrakingStyle};

use super::{Device, Event};

// ── Speed mapping (from utils.ts `mapSpeed`) ─────────────────────────────────

/// Clamp `speed` (-100..=100 or 127 for brake) to a valid wire byte.
fn map_speed(speed: i8) -> u8 {
    if speed == 127 {
        return 127;
    }
    speed.clamp(-100, 100).cast_unsigned()
}

// ── Profile flags ────────────────────────────────────────────────────────────

fn use_profile(accel: bool, decel: bool) -> u8 {
    u8::from(accel) | (u8::from(decel) << 1)
}

// ── BasicMotorDevice ─────────────────────────────────────────────────────────

/// Motor with power control only (`set_power`, `stop`, `brake`).
///
/// Corresponds to `BasicMotor` in the TS source.
pub struct BasicMotorDevice {
    port_id: u8,
    device_type_id: u16,
}

impl BasicMotorDevice {
    #[must_use]
    pub fn new(port_id: u8, device_type_id: u16) -> Self {
        Self {
            port_id,
            device_type_id,
        }
    }

    /// Encode a `WRITE_DIRECT_MODE_DATA` (0x51) payload for power control.
    ///
    /// Wire: `[0x51, mode=0x00, mapped_power]`
    #[must_use]
    pub fn encode_set_power(&self, power: i8) -> Bytes {
        Bytes::from(vec![0x51, 0x00, map_speed(power)])
    }

    /// Stop: set power to 0, interrupting any queued command.
    #[must_use]
    pub fn encode_stop(&self) -> Bytes {
        self.encode_set_power(0)
    }

    /// Brake: set power to `BrakingStyle::Brake` (127).
    #[must_use]
    pub fn encode_brake(&self) -> Bytes {
        self.encode_set_power(BrakingStyle::Brake as i8)
    }
}

impl Device for BasicMotorDevice {
    fn device_type_id(&self) -> u16 {
        self.device_type_id
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn receive(&mut self, _data: &[u8]) -> Result<Option<Event>> {
        Ok(None) // BasicMotor produces no sensor events
    }
}

// ── TachoMotorDevice ─────────────────────────────────────────────────────────

/// Motor with speed control and rotation-encoder feedback.
///
/// Corresponds to `TachoMotor` in the TS source.
pub struct TachoMotorDevice {
    port_id: u8,
    device_type_id: u16,
    mode: u8,
    brake_style: BrakingStyle,
    max_power: u8,
    pub use_acceleration_profile: bool,
    pub use_deceleration_profile: bool,
}

impl TachoMotorDevice {
    #[must_use]
    pub fn new(port_id: u8, device_type_id: u16) -> Self {
        Self {
            port_id,
            device_type_id,
            mode: 0,
            brake_style: BrakingStyle::Brake,
            max_power: 100,
            use_acceleration_profile: true,
            use_deceleration_profile: true,
        }
    }

    pub fn set_braking_style(&mut self, style: BrakingStyle) {
        self.brake_style = style;
    }

    pub fn set_max_power(&mut self, max_power: u8) {
        self.max_power = max_power;
    }

    /// Encode `SET_ACC_TIME` (subcommand 0x05).
    #[must_use]
    pub fn encode_set_acceleration_time(&self, time_ms: u16, profile: u8) -> Bytes {
        let [lo, hi] = time_ms.to_le_bytes();
        Bytes::from(vec![0x05, lo, hi, profile])
    }

    /// Encode `SET_DEC_TIME` (subcommand 0x06).
    #[must_use]
    pub fn encode_set_deceleration_time(&self, time_ms: u16, profile: u8) -> Bytes {
        let [lo, hi] = time_ms.to_le_bytes();
        Bytes::from(vec![0x06, lo, hi, profile])
    }

    /// Encode `START_SPEED` (subcommand 0x07, no time limit).
    #[must_use]
    pub fn encode_set_speed(&self, speed: i8) -> Bytes {
        Bytes::from(vec![
            0x07,
            map_speed(speed),
            self.max_power,
            use_profile(self.use_acceleration_profile, self.use_deceleration_profile),
        ])
    }

    /// Encode `START_SPEED_FOR_TIME` (subcommand 0x09).
    #[must_use]
    pub fn encode_set_speed_for_time(&self, speed: i8, time_ms: u16) -> Bytes {
        let [lo, hi] = time_ms.to_le_bytes();
        Bytes::from(vec![
            0x09,
            lo,
            hi,
            map_speed(speed),
            self.max_power,
            self.brake_style as u8,
            use_profile(self.use_acceleration_profile, self.use_deceleration_profile),
        ])
    }

    /// Encode `START_SPEED_FOR_DEGREES` (subcommand 0x0b).
    #[must_use]
    pub fn encode_rotate_by_degrees(&self, degrees: u32, speed: i8) -> Bytes {
        let deg = degrees.to_le_bytes();
        Bytes::from(vec![
            0x0b,
            deg[0],
            deg[1],
            deg[2],
            deg[3],
            map_speed(speed),
            self.max_power,
            self.brake_style as u8,
            use_profile(self.use_acceleration_profile, self.use_deceleration_profile),
        ])
    }
}

impl Device for TachoMotorDevice {
    fn device_type_id(&self) -> u16 {
        self.device_type_id
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn set_mode(&mut self, mode: u8) {
        self.mode = mode;
    }
    fn receive(&mut self, data: &[u8]) -> Result<Option<Event>> {
        // Mode 0x02 = ROTATION — 4-byte little-endian i32 at offset 0
        if self.mode == 0x02 && data.len() >= 4 {
            let degrees = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            return Ok(Some(Event::MotorRotate {
                port_id: self.port_id,
                degrees,
            }));
        }
        Ok(None)
    }
}

// ── AbsoluteMotorDevice ──────────────────────────────────────────────────────

/// Motor with absolute-position control.
///
/// Corresponds to `AbsoluteMotor` in the TS source.
pub struct AbsoluteMotorDevice {
    port_id: u8,
    device_type_id: u16,
    mode: u8,
    brake_style: BrakingStyle,
    max_power: u8,
    pub use_acceleration_profile: bool,
    pub use_deceleration_profile: bool,
}

impl AbsoluteMotorDevice {
    #[must_use]
    pub fn new(port_id: u8, device_type_id: u16) -> Self {
        Self {
            port_id,
            device_type_id,
            mode: 0,
            brake_style: BrakingStyle::Brake,
            max_power: 100,
            use_acceleration_profile: true,
            use_deceleration_profile: true,
        }
    }

    pub fn set_braking_style(&mut self, style: BrakingStyle) {
        self.brake_style = style;
    }

    pub fn set_max_power(&mut self, max_power: u8) {
        self.max_power = max_power;
    }

    /// Encode `GOTO_ABSOLUTE_POSITION` (subcommand 0x0d).
    #[must_use]
    pub fn encode_goto_angle(&self, angle: i32, speed: i8) -> Bytes {
        let ang = angle.to_le_bytes();
        Bytes::from(vec![
            0x0d,
            ang[0],
            ang[1],
            ang[2],
            ang[3],
            map_speed(speed),
            self.max_power,
            self.brake_style as u8,
            use_profile(self.use_acceleration_profile, self.use_deceleration_profile),
        ])
    }

    /// Encode `PRESET_ENCODER` (subcommand 0x51, mode 0x02) — reset zero.
    #[must_use]
    pub fn encode_reset_zero(&self) -> Bytes {
        Bytes::from(vec![0x51, 0x02, 0x00, 0x00, 0x00, 0x00])
    }

    /// Encode `START_SPEED_FOR_DEGREES` — inherited from tacho level.
    #[must_use]
    pub fn encode_rotate_by_degrees(&self, degrees: u32, speed: i8) -> Bytes {
        let deg = degrees.to_le_bytes();
        Bytes::from(vec![
            0x0b,
            deg[0],
            deg[1],
            deg[2],
            deg[3],
            map_speed(speed),
            self.max_power,
            self.brake_style as u8,
            use_profile(self.use_acceleration_profile, self.use_deceleration_profile),
        ])
    }
}

impl Device for AbsoluteMotorDevice {
    fn device_type_id(&self) -> u16 {
        self.device_type_id
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn set_mode(&mut self, mode: u8) {
        self.mode = mode;
    }
    fn receive(&mut self, data: &[u8]) -> Result<Option<Event>> {
        match self.mode {
            // Mode 0x02 = ROTATION — i32 LE
            0x02 if data.len() >= 4 => {
                let degrees = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                Ok(Some(Event::MotorRotate {
                    port_id: self.port_id,
                    degrees,
                }))
            }
            // Mode 0x03 = ABSOLUTE — i16 LE
            0x03 if data.len() >= 2 => {
                let angle = i16::from_le_bytes([data[0], data[1]]);
                Ok(Some(Event::MotorAngle {
                    port_id: self.port_id,
                    angle,
                }))
            }
            _ => Ok(None),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── BasicMotorDevice ──────────────────────────────────────────────────────

    #[test]
    fn basic_motor_set_power_positive() {
        let dev = BasicMotorDevice::new(0, 2);
        assert_eq!(dev.encode_set_power(50).as_ref(), &[0x51, 0x00, 50]);
    }

    #[test]
    fn basic_motor_set_power_negative() {
        let dev = BasicMotorDevice::new(0, 2);
        // -50 as i8 → 0xCE as u8
        let expected = (-50i8) as u8;
        assert_eq!(dev.encode_set_power(-50).as_ref(), &[0x51, 0x00, expected]);
    }

    #[test]
    fn basic_motor_brake_encodes_127() {
        let dev = BasicMotorDevice::new(0, 2);
        assert_eq!(dev.encode_brake().as_ref(), &[0x51, 0x00, 127]);
    }

    #[test]
    fn basic_motor_stop_encodes_zero() {
        let dev = BasicMotorDevice::new(0, 2);
        assert_eq!(dev.encode_stop().as_ref(), &[0x51, 0x00, 0]);
    }

    // ── TachoMotorDevice ──────────────────────────────────────────────────────

    #[test]
    fn tacho_set_speed_encodes_correctly() {
        let dev = TachoMotorDevice::new(1, 38);
        // speed=50, max_power=100, profile=0b11 (both profiles on)
        assert_eq!(dev.encode_set_speed(50).as_ref(), &[0x07, 50, 100, 0x03]);
    }

    #[test]
    fn tacho_set_speed_for_time() {
        let dev = TachoMotorDevice::new(1, 38);
        // time=1000ms=0x03E8, speed=50, max_power=100, brake=127, profile=0x03
        let bytes = dev.encode_set_speed_for_time(50, 1000);
        assert_eq!(bytes.as_ref(), &[0x09, 0xE8, 0x03, 50, 100, 127, 0x03]);
    }

    #[test]
    fn tacho_rotate_by_degrees() {
        let dev = TachoMotorDevice::new(0, 38);
        // degrees=360=0x00000168, speed=100, max_power=100, brake=127, profile=0x03
        let bytes = dev.encode_rotate_by_degrees(360, 100);
        assert_eq!(
            bytes.as_ref(),
            &[0x0b, 0x68, 0x01, 0x00, 0x00, 100, 100, 127, 0x03]
        );
    }

    #[test]
    fn tacho_receive_rotation_mode() {
        let mut dev = TachoMotorDevice::new(0, 38);
        dev.set_mode(0x02);
        // 360 degrees as i32 LE
        let data = 360i32.to_le_bytes();
        let event = dev.receive(&data).unwrap().unwrap();
        if let Event::MotorRotate { port_id, degrees } = event {
            assert_eq!(port_id, 0);
            assert_eq!(degrees, 360);
        } else {
            panic!("expected MotorRotate");
        }
    }

    // ── AbsoluteMotorDevice ───────────────────────────────────────────────────

    #[test]
    fn absolute_goto_angle() {
        let dev = AbsoluteMotorDevice::new(0, 48);
        // angle=90, speed=100, max_power=100, brake=127, profile=0x03
        let bytes = dev.encode_goto_angle(90, 100);
        assert_eq!(
            bytes.as_ref(),
            &[0x0d, 90, 0x00, 0x00, 0x00, 100, 100, 127, 0x03]
        );
    }

    #[test]
    fn absolute_reset_zero() {
        let dev = AbsoluteMotorDevice::new(0, 48);
        assert_eq!(
            dev.encode_reset_zero().as_ref(),
            &[0x51, 0x02, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn absolute_receive_rotation_mode() {
        let mut dev = AbsoluteMotorDevice::new(0, 48);
        dev.set_mode(0x02);
        let data = (-180i32).to_le_bytes();
        let event = dev.receive(&data).unwrap().unwrap();
        if let Event::MotorRotate { degrees, .. } = event {
            assert_eq!(degrees, -180);
        } else {
            panic!("expected MotorRotate");
        }
    }

    #[test]
    fn absolute_receive_angle_mode() {
        let mut dev = AbsoluteMotorDevice::new(0, 48);
        dev.set_mode(0x03);
        let data = 90i16.to_le_bytes();
        let event = dev.receive(&data).unwrap().unwrap();
        if let Event::MotorAngle { angle, .. } = event {
            assert_eq!(angle, 90);
        } else {
            panic!("expected MotorAngle");
        }
    }
}
