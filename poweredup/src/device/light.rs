//! Light and audio actuator devices — ported from `hubLED.ts`, `light.ts`,
//! `piezobuzzer.ts`, `technic3x3colorlightmatrix.ts`.
//!
//! These are actuators only — `receive` always returns `Ok(None)`.
//! Command encoders return [`Bytes`] for the hub layer to transmit via
//! `writeDirect` (LPF2 sub-command 0x51).

use bytes::Bytes;

use crate::{error::Result, protocol::consts::Color};

use super::{Device, Event};

// ── HubLed ────────────────────────────────────────────────────────────────────

/// The status LED on the hub (device type 23).
pub struct HubLed {
    port_id: u8,
}

impl HubLed {
    #[must_use]
    pub fn new(port_id: u8) -> Self {
        Self { port_id }
    }

    /// Set the LED to a named [`Color`].
    /// Wire: `WRITE_DIRECT_MODE_DATA [0x51, mode=0x00, color_index]`
    #[must_use]
    pub fn encode_set_color(&self, color: Color) -> Bytes {
        Bytes::from(vec![0x51, 0x00, color as u8])
    }

    /// Set the LED to an arbitrary RGB value.
    /// Wire: `WRITE_DIRECT_MODE_DATA [0x51, mode=0x01, r, g, b]`
    #[must_use]
    pub fn encode_set_rgb(&self, red: u8, green: u8, blue: u8) -> Bytes {
        Bytes::from(vec![0x51, 0x01, red, green, blue])
    }
}

impl Device for HubLed {
    fn device_type_id(&self) -> u16 {
        23
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn receive(&mut self, _data: &[u8]) -> Result<Option<Event>> {
        Ok(None)
    }
}

// ── Light ─────────────────────────────────────────────────────────────────────

/// External light / LED strip (device type 8).
pub struct Light {
    port_id: u8,
}

impl Light {
    #[must_use]
    pub fn new(port_id: u8) -> Self {
        Self { port_id }
    }

    /// Set brightness 0–100.
    /// Wire: `WRITE_DIRECT_MODE_DATA [0x51, mode=0x00, brightness]`
    #[must_use]
    pub fn encode_set_brightness(&self, brightness: u8) -> Bytes {
        Bytes::from(vec![0x51, 0x00, brightness.min(100)])
    }
}

impl Device for Light {
    fn device_type_id(&self) -> u16 {
        8
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn receive(&mut self, _data: &[u8]) -> Result<Option<Event>> {
        Ok(None)
    }
}

// ── PiezoBuzzer ───────────────────────────────────────────────────────────────

/// LEGO `WeDo 2.0` piezo buzzer (device type 22).
///
/// Note: unlike LPF2 devices, the buzzer payload is sent directly to the
/// `WEDO2_MOTOR_VALUE_WRITE` characteristic rather than via `PortOutputCommand`.
/// The encoded bytes here match the raw wire format used by the `WeDo 2.0` hub:
/// `[0x05, 0x02, 0x04, freq_lo, freq_hi, time_lo, time_hi]`
pub struct PiezoBuzzer {
    port_id: u8,
}

impl PiezoBuzzer {
    #[must_use]
    pub fn new(port_id: u8) -> Self {
        Self { port_id }
    }

    /// Encode a tone command: frequency in Hz, duration in milliseconds.
    #[must_use]
    pub fn encode_play_tone(&self, frequency: u16, time_ms: u16) -> Bytes {
        let [f_lo, f_hi] = frequency.to_le_bytes();
        let [t_lo, t_hi] = time_ms.to_le_bytes();
        Bytes::from(vec![0x05, 0x02, 0x04, f_lo, f_hi, t_lo, t_hi])
    }
}

impl Device for PiezoBuzzer {
    fn device_type_id(&self) -> u16 {
        22
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn receive(&mut self, _data: &[u8]) -> Result<Option<Event>> {
        Ok(None)
    }
}

// ── Technic3x3ColorLightMatrix ────────────────────────────────────────────────

/// 3×3 colour light matrix (device type 64 — Spike Essential).
///
/// Each of the 9 pixels is encoded as `color_index | (brightness << 4)`.
/// Maximum brightness is 10 (0x0A).
pub struct ColorLightMatrix {
    port_id: u8,
}

impl ColorLightMatrix {
    #[must_use]
    pub fn new(port_id: u8) -> Self {
        Self { port_id }
    }

    /// Set all 9 pixels to the same color at maximum brightness.
    /// Wire: `WRITE_DIRECT_MODE_DATA [0x51, mode=0x02, pixel×9]`
    #[must_use]
    pub fn encode_set_color(&self, color: Color) -> Bytes {
        let pixel = (color as u8) | (10 << 4);
        let mut v = vec![0x51, 0x02];
        v.extend(std::iter::repeat_n(pixel, 9));
        Bytes::from(v)
    }

    /// Set each pixel individually.  `colors` must contain exactly 9 elements;
    /// any excess is silently truncated.
    /// Brightness is always set to max (10) per the TS implementation.
    #[must_use]
    pub fn encode_set_matrix(&self, colors: &[Color]) -> Bytes {
        let pixels: Vec<u8> = colors
            .iter()
            .take(9)
            .map(|&c| (c as u8) | (10 << 4))
            .collect();
        let mut v = vec![0x51, 0x02];
        v.extend_from_slice(&pixels);
        Bytes::from(v)
    }
}

impl Device for ColorLightMatrix {
    fn device_type_id(&self) -> u16 {
        64
    }
    fn port_id(&self) -> u8 {
        self.port_id
    }
    fn receive(&mut self, _data: &[u8]) -> Result<Option<Event>> {
        Ok(None)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_led_set_color() {
        let dev = HubLed::new(50); // hub LED is typically on port 50
        let bytes = dev.encode_set_color(Color::Red);
        assert_eq!(bytes.as_ref(), &[0x51, 0x00, Color::Red as u8]);
    }

    #[test]
    fn hub_led_set_rgb() {
        let dev = HubLed::new(50);
        let bytes = dev.encode_set_rgb(255, 0, 128);
        assert_eq!(bytes.as_ref(), &[0x51, 0x01, 255, 0, 128]);
    }

    #[test]
    fn light_set_brightness_clamps_to_100() {
        let dev = Light::new(0);
        assert_eq!(dev.encode_set_brightness(150).as_ref(), &[0x51, 0x00, 100]);
        assert_eq!(dev.encode_set_brightness(50).as_ref(), &[0x51, 0x00, 50]);
    }

    #[test]
    fn piezo_buzzer_encode_tone() {
        let dev = PiezoBuzzer::new(0);
        // frequency=440 Hz (0x01B8 LE), time=500 ms (0x01F4 LE)
        let bytes = dev.encode_play_tone(440, 500);
        assert_eq!(bytes.as_ref(), &[0x05, 0x02, 0x04, 0xB8, 0x01, 0xF4, 0x01]);
    }

    #[test]
    fn color_matrix_set_uniform_color() {
        let dev = ColorLightMatrix::new(0);
        let bytes = dev.encode_set_color(Color::Blue);
        // pixel = Blue(3) | (10 << 4) = 0x03 | 0xA0 = 0xA3
        let expected_pixel = (Color::Blue as u8) | (10 << 4);
        assert_eq!(bytes[0], 0x51);
        assert_eq!(bytes[1], 0x02);
        assert!(bytes[2..].iter().all(|&b| b == expected_pixel));
        assert_eq!(bytes.len(), 11); // header(2) + 9 pixels
    }

    #[test]
    fn color_matrix_set_matrix() {
        let dev = ColorLightMatrix::new(0);
        let colors = [
            Color::Red, Color::Green, Color::Blue,
            Color::Red, Color::Green, Color::Blue,
            Color::Red, Color::Green, Color::Blue,
        ];
        let bytes = dev.encode_set_matrix(&colors);
        assert_eq!(bytes.len(), 11);
        assert_eq!(bytes[0], 0x51);
        assert_eq!(bytes[2], (Color::Red as u8) | (10 << 4));
        assert_eq!(bytes[3], (Color::Green as u8) | (10 << 4));
        assert_eq!(bytes[4], (Color::Blue as u8) | (10 << 4));
    }
}
