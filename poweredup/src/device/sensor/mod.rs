//! Sensor device types — ported from the TS device source files.
//!
//! Each struct implements [`Device`] and decodes `PORT_VALUE_SINGLE` payloads
//! into typed [`Event`] variants.  The active sensor mode is tracked via
//! [`Device::set_mode`] so the correct decode path is applied.

use crate::{error::Result, protocol::consts::Color};

use super::{Device, Event};

// ── ColorDistanceSensor ───────────────────────────────────────────────────────

/// Mode discriminants for `ColorDistanceSensor` (device type 0x25 = 37).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDistanceMode {
    Color = 0x00,
    Distance = 0x01,
    DistanceCount = 0x02,
    Reflect = 0x03,
    Ambient = 0x04,
    RgbIntensity = 0x05,
    ColorAndDistance = 0x08,
}

pub struct ColorDistanceSensor {
    port_id: u8,
    mode: u8,
}

impl ColorDistanceSensor {
    #[must_use]
    pub fn new(port_id: u8) -> Self {
        Self { port_id, mode: 0 }
    }
}

impl Device for ColorDistanceSensor {
    fn device_type_id(&self) -> u16 {
        37
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn set_mode(&mut self, mode: u8) {
        self.mode = mode;
    }
    fn receive(&mut self, data: &[u8]) -> Result<Option<Event>> {
        match self.mode {
            // 0x00 COLOR — 1 byte color index
            m if m == ColorDistanceMode::Color as u8 => {
                if let Some(&raw) = data.first()
                    && raw <= 10
                    && let Ok(color) = Color::try_from(raw)
                {
                    return Ok(Some(Event::Color {
                        port_id: self.port_id,
                        color,
                    }));
                }
                Ok(None)
            }
            // 0x01 DISTANCE — 1 byte, in units of ~25.4 mm
            m if m == ColorDistanceMode::Distance as u8 => {
                if let Some(&raw) = data.first()
                    && raw <= 10
                {
                    let mm = (i32::from(raw) * 254 / 10 - 20).max(0).cast_unsigned();
                    return Ok(Some(Event::Distance {
                        port_id: self.port_id,
                        distance_mm: mm,
                    }));
                }
                Ok(None)
            }
            // 0x03 REFLECT — 1 byte 0-100%
            m if m == ColorDistanceMode::Reflect as u8 => {
                Ok(data.first().map(|&r| Event::Reflect {
                    port_id: self.port_id,
                    percent: r,
                }))
            }
            // 0x04 AMBIENT — 1 byte 0-100%
            m if m == ColorDistanceMode::Ambient as u8 => {
                Ok(data.first().map(|&a| Event::Ambient {
                    port_id: self.port_id,
                    percent: a,
                }))
            }
            // 0x08 COLOR_AND_DISTANCE — bytes: [color, ?, dist, ?, partial, ...]
            m if m == ColorDistanceMode::ColorAndDistance as u8 => {
                if data.len() >= 4 {
                    let color_raw = data[0];
                    if color_raw <= 10 {
                        let dist_raw = f32::from(data[2]);
                        let partial = data[3];
                        let dist = if partial > 0 {
                            dist_raw + 1.0 / f32::from(partial)
                        } else {
                            dist_raw
                        };
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let mm = ((dist * 25.4) as i32 - 20).max(0).cast_unsigned();
                        if let Ok(color) = Color::try_from(color_raw) {
                            return Ok(Some(Event::ColorAndDistance {
                                port_id: self.port_id,
                                color,
                                distance_mm: mm,
                            }));
                        }
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

// ── MotionSensor ──────────────────────────────────────────────────────────────

pub struct MotionSensor {
    port_id: u8,
}

impl MotionSensor {
    #[must_use]
    pub fn new(port_id: u8) -> Self {
        Self { port_id }
    }
}

impl Device for MotionSensor {
    fn device_type_id(&self) -> u16 {
        35
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn receive(&mut self, data: &[u8]) -> Result<Option<Event>> {
        // Mode 0x00 DISTANCE: byte[0] base, byte[1] overflow flag
        if data.len() >= 2 {
            let base = u32::from(data[0]);
            let overflow = u32::from(data[1]);
            let mm = (base + overflow * 255) * 10;
            return Ok(Some(Event::Distance {
                port_id: self.port_id,
                distance_mm: mm,
            }));
        }
        Ok(None)
    }
}

// ── TiltSensor ────────────────────────────────────────────────────────────────

pub struct TiltSensor {
    port_id: u8,
    mode: u8,
}

impl TiltSensor {
    #[must_use]
    pub fn new(port_id: u8) -> Self {
        Self { port_id, mode: 0 }
    }
}

impl Device for TiltSensor {
    fn device_type_id(&self) -> u16 {
        34
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn set_mode(&mut self, mode: u8) {
        self.mode = mode;
    }
    fn receive(&mut self, data: &[u8]) -> Result<Option<Event>> {
        match self.mode {
            // 0x00 TILT — x, y as i8
            0x00 if data.len() >= 2 => Ok(Some(Event::Tilt {
                port_id: self.port_id,
                x: data[0].cast_signed(),
                y: data[1].cast_signed(),
                z: None,
            })),
            _ => Ok(None),
        }
    }
}

// ── VoltageSensor ─────────────────────────────────────────────────────────────

/// Voltage sensor (device type 20).
///
/// The TS source applies per-hub scaling; here we emit the raw millivolt value
/// computed from the standard (non-WeDo2) path with the default 9615 mV max /
/// 3893 raw-count scale.  Hub-subclass layers can post-process if needed.
pub struct VoltageSensor {
    port_id: u8,
}

impl VoltageSensor {
    #[must_use]
    pub fn new(port_id: u8) -> Self {
        Self { port_id }
    }
}

impl Device for VoltageSensor {
    fn device_type_id(&self) -> u16 {
        20
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn receive(&mut self, data: &[u8]) -> Result<Option<Event>> {
        if data.len() >= 2 {
            let raw = f32::from(u16::from_le_bytes([data[0], data[1]]));
            // default scale: 9615 mV / 3893 counts
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let mv = (raw * 9615.0 / 3893.0) as u32;
            return Ok(Some(Event::Voltage {
                port_id: self.port_id,
                millivolts: mv,
            }));
        }
        Ok(None)
    }
}

// ── CurrentSensor ─────────────────────────────────────────────────────────────

/// Current sensor (device type 21).
///
/// Default scale: 2444 mA / 4095 counts.
pub struct CurrentSensor {
    port_id: u8,
}

impl CurrentSensor {
    #[must_use]
    pub fn new(port_id: u8) -> Self {
        Self { port_id }
    }
}

impl Device for CurrentSensor {
    fn device_type_id(&self) -> u16 {
        21
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn receive(&mut self, data: &[u8]) -> Result<Option<Event>> {
        if data.len() >= 2 {
            let raw = f32::from(u16::from_le_bytes([data[0], data[1]]));
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let ma = (raw * 2444.0 / 4095.0) as u32;
            return Ok(Some(Event::Current {
                port_id: self.port_id,
                milliamps: ma,
            }));
        }
        Ok(None)
    }
}

// ── RemoteControlButton ───────────────────────────────────────────────────────

/// Remote-control button state wire values.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteButtonState {
    Released = 0x00,
    Up = 0x01,
    Down = 0xFF,
    Stop = 0x7F,
}

pub struct RemoteControlButton {
    port_id: u8,
}

impl RemoteControlButton {
    #[must_use]
    pub fn new(port_id: u8) -> Self {
        Self { port_id }
    }
}

impl Device for RemoteControlButton {
    fn device_type_id(&self) -> u16 {
        55
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn receive(&mut self, data: &[u8]) -> Result<Option<Event>> {
        Ok(data.first().map(|&raw| Event::RemoteButton {
            port_id: self.port_id,
            state: raw,
        }))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_distance_color_mode() {
        let mut dev = ColorDistanceSensor::new(0);
        dev.set_mode(ColorDistanceMode::Color as u8);
        let event = dev.receive(&[3]).unwrap().unwrap(); // color index 3 = Blue
        if let Event::Color { color, .. } = event {
            assert_eq!(color, Color::Blue);
        } else {
            panic!("expected Color event");
        }
    }

    #[test]
    fn color_distance_distance_mode() {
        let mut dev = ColorDistanceSensor::new(0);
        dev.set_mode(ColorDistanceMode::Distance as u8);
        // raw=5 → 5*254/10-20 = 127-20 = 107 mm
        let event = dev.receive(&[5]).unwrap().unwrap();
        if let Event::Distance { distance_mm, .. } = event {
            assert_eq!(distance_mm, 107);
        } else {
            panic!("expected Distance event");
        }
    }

    #[test]
    fn motion_sensor_distance() {
        let mut dev = MotionSensor::new(1);
        // base=2, overflow=0 → 20 mm
        let event = dev.receive(&[2, 0]).unwrap().unwrap();
        if let Event::Distance { distance_mm, .. } = event {
            assert_eq!(distance_mm, 20);
        } else {
            panic!("expected Distance event");
        }
    }

    #[test]
    fn voltage_sensor_decodes() {
        let mut dev = VoltageSensor::new(0);
        // raw=3893 → 9615 mV
        let data = 3893u16.to_le_bytes();
        let event = dev.receive(&data).unwrap().unwrap();
        if let Event::Voltage { millivolts, .. } = event {
            assert_eq!(millivolts, 9615);
        } else {
            panic!("expected Voltage event");
        }
    }

    #[test]
    fn current_sensor_decodes() {
        let mut dev = CurrentSensor::new(0);
        // raw=4095 → 2444 mA
        let data = 4095u16.to_le_bytes();
        let event = dev.receive(&data).unwrap().unwrap();
        if let Event::Current { milliamps, .. } = event {
            assert_eq!(milliamps, 2444);
        } else {
            panic!("expected Current event");
        }
    }

    #[test]
    fn remote_button_encodes_state() {
        let mut dev = RemoteControlButton::new(2);
        let event = dev.receive(&[0x01]).unwrap().unwrap();
        if let Event::RemoteButton { state, .. } = event {
            assert_eq!(state, RemoteButtonState::Up as u8);
        } else {
            panic!("expected RemoteButton event");
        }
    }
}

