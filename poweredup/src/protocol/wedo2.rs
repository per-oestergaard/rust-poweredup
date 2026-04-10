//! `WeDo 2.0` Smart Hub protocol codec.
//!
//! The `WeDo 2.0` Smart Hub uses a completely different BLE protocol from LPF2
//! (Powered UP).  Instead of a single multiplexed characteristic, it exposes
//! several dedicated characteristics:
//!
//! | Direction    | Characteristic UUID (const)   | Purpose                    |
//! |--------------|-------------------------------|----------------------------|
//! | Upstream     | `WEDO2_PORT_TYPE` (0x1527)    | Device attach / detach     |
//! | Upstream     | `WEDO2_SENSOR_VALUE` (0x1560) | Sensor readings            |
//! | Upstream     | `WEDO2_BUTTON` (0x1526)       | Hub button                 |
//! | Downstream   | `WEDO2_PORT_TYPE_WRITE` (0x1563) | Subscribe / unsubscribe |
//! | Downstream   | `WEDO2_MOTOR_WRITE` (0x1565)  | Motor power                |
//! | Downstream   | `WEDO2_DISCONNECT` (0x152b)   | Graceful disconnect        |
//! | Downstream   | `WEDO2_NAME_ID` (0x1524)      | Set hub name               |
//!
//! This module handles parse and encode for each of these characteristics.
//! The actual BLE `subscribe` / `write` calls are performed by the hub layer.

use bytes::Bytes;

use crate::error::{Error, Result};

// ── Port attach / detach ──────────────────────────────────────────────────────

/// Possible device event types reported on `WEDO2_PORT_TYPE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wedo2IoEvent {
    /// A device was plugged into the port.
    Attached,
    /// A device was unplugged from the port.
    Detached,
}

/// Parsed message from the `WEDO2_PORT_TYPE` (0x1527) characteristic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wedo2PortMessage {
    /// Port identifier (1 = port A, 2 = port B on the Smart Hub).
    pub port_id: u8,
    /// Whether a device was attached or detached.
    pub event: Wedo2IoEvent,
    /// Device type ID when `event == Attached`; `0` otherwise.
    pub device_type_id: u8,
}

/// Parse a raw notification from the `WEDO2_PORT_TYPE` characteristic.
///
/// Format: `[port_id, event, ?, device_type_id, ...]`
/// where `event` is `0x01` for attach, `0x00` for detach.
///
/// # Errors
/// Returns [`Error::Parse`] when the payload is shorter than 2 bytes.
pub fn parse_port_message(data: &[u8]) -> Result<Wedo2PortMessage> {
    if data.len() < 2 {
        return Err(Error::Parse(format!(
            "wedo2 port message too short: {} bytes",
            data.len()
        )));
    }
    let port_id = data[0];
    let event = match data[1] {
        0x01 => Wedo2IoEvent::Attached,
        0x00 => Wedo2IoEvent::Detached,
        other => {
            return Err(Error::Parse(format!(
                "unknown wedo2 port event: {other:#x}"
            )))
        }
    };
    let device_type_id = if event == Wedo2IoEvent::Attached {
        data.get(3).copied().unwrap_or(0)
    } else {
        0
    };
    Ok(Wedo2PortMessage {
        port_id,
        event,
        device_type_id,
    })
}

// ── Button / sensor values ────────────────────────────────────────────────────

/// Parsed message from the `WEDO2_SENSOR_VALUE` (0x1560) or `WEDO2_BUTTON`
/// (0x1526) characteristics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wedo2SensorMessage {
    /// Hub button state.
    Button {
        /// `true` when the button is pressed.
        pressed: bool,
    },
    /// Raw sensor payload for the given port.
    PortData {
        /// Port identifier (1 = A, 2 = B).
        port_id: u8,
        /// Raw sensor bytes (everything after the `port_id` byte).
        data: Bytes,
    },
}

/// Parse a raw notification from `WEDO2_SENSOR_VALUE` or `WEDO2_BUTTON`.
///
/// Format:
/// * `[0x01]` — button pressed
/// * `[0x00]` — button released
/// * `[?, port_id, data…]` — sensor data for `port_id`
///
/// # Errors
/// Returns [`Error::Parse`] when the payload is empty.
pub fn parse_sensor_message(data: &[u8]) -> Result<Wedo2SensorMessage> {
    match data {
        [] => Err(Error::Parse("empty wedo2 sensor message".into())),
        [0x01] => Ok(Wedo2SensorMessage::Button { pressed: true }),
        [0x00] => Ok(Wedo2SensorMessage::Button { pressed: false }),
        [_, port_id, rest @ ..] => Ok(Wedo2SensorMessage::PortData {
            port_id: *port_id,
            data: Bytes::copy_from_slice(rest),
        }),
        _ => Ok(Wedo2SensorMessage::Button {
            pressed: data[0] != 0,
        }),
    }
}

// ── Downstream command encoding ───────────────────────────────────────────────

/// Encode a **subscribe** command for `WEDO2_PORT_TYPE_WRITE` (0x1563).
///
/// Instructs the hub to start sending sensor notifications for the given port.
/// `mode` selects which sensor mode to use (device-specific).
#[must_use]
pub fn encode_subscribe(port_id: u8, device_type: u8, mode: u8) -> Bytes {
    Bytes::copy_from_slice(&[
        0x01, 0x02, port_id, device_type, mode, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
    ])
}

/// Encode an **unsubscribe** command for `WEDO2_PORT_TYPE_WRITE` (0x1563).
///
/// Instructs the hub to stop sending sensor notifications for the given port.
#[must_use]
pub fn encode_unsubscribe(port_id: u8, device_type: u8, mode: u8) -> Bytes {
    Bytes::copy_from_slice(&[
        0x01, 0x02, port_id, device_type, mode, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
    ])
}

/// Encode a **disconnect** command for `WEDO2_DISCONNECT` (0x152b).
///
/// Writing this to the disconnect characteristic causes the hub to terminate
/// the BLE connection cleanly.
#[must_use]
pub fn encode_disconnect() -> Bytes {
    Bytes::from_static(&[0x00])
}

/// Encode a **set name** command for `WEDO2_NAME_ID` (0x1524).
///
/// `name` must be ASCII and at most 14 characters; longer names are silently
/// truncated to 14 characters.
#[must_use]
pub fn encode_set_name(name: &str) -> Bytes {
    let truncated: String = name.chars().take(14).collect();
    Bytes::from(truncated.into_bytes())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_port_message ────────────────────────────────────────────────────

    #[test]
    fn parse_port_attach() {
        // [port=1, event=attach, ?, device_type=1(SimpleMediumLinearMotor)]
        let data = [0x01, 0x01, 0x00, 0x01, 0x00];
        let msg = parse_port_message(&data).unwrap();
        assert_eq!(msg.port_id, 1);
        assert_eq!(msg.event, Wedo2IoEvent::Attached);
        assert_eq!(msg.device_type_id, 1);
    }

    #[test]
    fn parse_port_detach() {
        let data = [0x02, 0x00];
        let msg = parse_port_message(&data).unwrap();
        assert_eq!(msg.port_id, 2);
        assert_eq!(msg.event, Wedo2IoEvent::Detached);
        assert_eq!(msg.device_type_id, 0);
    }

    #[test]
    fn parse_port_message_too_short_errors() {
        assert!(parse_port_message(&[0x01]).is_err());
        assert!(parse_port_message(&[]).is_err());
    }

    #[test]
    fn parse_port_unknown_event_errors() {
        assert!(parse_port_message(&[0x01, 0x02]).is_err());
    }

    // ── parse_sensor_message ──────────────────────────────────────────────────

    #[test]
    fn parse_button_pressed() {
        let msg = parse_sensor_message(&[0x01]).unwrap();
        assert_eq!(msg, Wedo2SensorMessage::Button { pressed: true });
    }

    #[test]
    fn parse_button_released() {
        let msg = parse_sensor_message(&[0x00]).unwrap();
        assert_eq!(msg, Wedo2SensorMessage::Button { pressed: false });
    }

    #[test]
    fn parse_sensor_port_data() {
        // [header_byte, port_id=1, value_byte=42]
        let data = [0x05, 0x01, 0x2A];
        let msg = parse_sensor_message(&data).unwrap();
        assert_eq!(
            msg,
            Wedo2SensorMessage::PortData {
                port_id: 1,
                data: Bytes::from_static(&[0x2A]),
            }
        );
    }

    #[test]
    fn parse_sensor_empty_errors() {
        assert!(parse_sensor_message(&[]).is_err());
    }

    // ── encode_subscribe / unsubscribe ────────────────────────────────────────

    #[test]
    fn encode_subscribe_roundtrip() {
        let sub = encode_subscribe(1, 8, 0); // port A, HubLED, mode 0
        assert_eq!(sub.len(), 11);
        assert_eq!(&sub[..], &[0x01, 0x02, 0x01, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01]);

        let unsub = encode_unsubscribe(1, 8, 0);
        assert_eq!(unsub.len(), 11);
        // Last byte is 0x00 for unsubscribe
        assert_eq!(unsub[10], 0x00);
        // Everything else is the same
        assert_eq!(&sub[..10], &unsub[..10]);
    }

    #[test]
    fn encode_disconnect_is_single_zero_byte() {
        let disc = encode_disconnect();
        assert_eq!(&disc[..], &[0x00]);
    }

    #[test]
    fn encode_set_name_truncates_at_14_chars() {
        let long_name = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"; // 26 chars
        let encoded = encode_set_name(long_name);
        assert_eq!(encoded.len(), 14);
        assert_eq!(&encoded[..], b"ABCDEFGHIJKLMN");
    }

    #[test]
    fn encode_set_name_short() {
        let encoded = encode_set_name("Hub");
        assert_eq!(&encoded[..], b"Hub");
    }
}
