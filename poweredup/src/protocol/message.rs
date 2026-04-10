//! LPF2 message codec — the parse-not-validate boundary.
//!
//! Raw `&[u8]` from the BLE notification stream enters via [`parse`] and exits
//! as a typed [`LpfMessage`]; commands leave via [`encode`] as [`bytes::Bytes`].
//! No raw bytes ever cross this module boundary into the hub or device layers.
//!
//! Wire format (little-endian):
//! ```text
//! [0]        length   – total message length including this byte
//! [1]        0x00     – reserved / hub ID (always 0x00 for Powered UP)
//! [2]        type     – MessageType discriminant
//! [3..]      payload  – type-specific bytes
//! ```
//!
//! Reference: <https://lego.github.io/lego-ble-wireless-protocol-docs/>

use bytes::{Bytes, BytesMut};

use super::consts::{
    ActionType, AlertPayload, AlertType, CommandFeedback, HubPropertyOperation,
    HubPropertyReference, IoEvent, MessageType,
};
use crate::error::{Error, Result};

// ── Top-level message type ────────────────────────────────────────────────────

/// A fully-parsed LPF2 upstream or downstream message.
#[derive(Debug, Clone, PartialEq)]
pub enum LpfMessage {
    /// Hub property report or request (0x01).
    HubProperty(HubPropertyMessage),
    /// Hub action command or notification (0x02).
    HubAction(ActionType),
    /// Hub alert notification (0x03).
    HubAlert(HubAlertMessage),
    /// Port attach / detach event (0x04).
    HubAttachedIo(HubAttachedIoMessage),
    /// Hub-to-host error response (0x05).
    GenericError { command: u8, code: u8 },
    /// Subscribe / unsubscribe a port to a sensor mode (0x41).
    PortInputFormatSetupSingle(PortInputFormatSetupSingle),
    /// Sensor value notification (0x45).
    PortValueSingle(PortValueSingle),
    /// Active mode notification (0x47).
    PortInputFormatSingle(PortInputFormatSingle),
    /// Motor / actuator command (0x81).
    PortOutputCommand(PortOutputCommand),
    /// Command completion feedback (0x82).
    PortOutputCommandFeedback(PortOutputCommandFeedback),
    /// Message type we received but do not yet decode — not an error.
    Unknown { message_type: u8, payload: Bytes },
}

// ── Sub-message types ─────────────────────────────────────────────────────────

/// Parsed `HUB_PROPERTIES` message.
#[derive(Debug, Clone, PartialEq)]
pub struct HubPropertyMessage {
    pub property: HubPropertyReference,
    pub operation: HubPropertyOperation,
    /// Raw payload bytes after the operation byte (type-dependent).
    pub payload: Bytes,
}

/// Parsed `HUB_ALERTS` message.
#[derive(Debug, Clone, PartialEq)]
pub struct HubAlertMessage {
    pub alert_type: AlertType,
    pub payload: AlertPayload,
}

/// Parsed `HUB_ATTACHED_IO` message.
#[derive(Debug, Clone, PartialEq)]
pub struct HubAttachedIoMessage {
    pub port_id: u8,
    pub event: IoEvent,
    /// `Some((device_type_id, hw_ver, sw_ver))` when `event == AttachedIo`.
    pub attached: Option<AttachedIoInfo>,
    /// `Some((first_port, second_port, device_type_id))` when `event == AttachedVirtualIo`.
    pub virtual_attached: Option<VirtualIoInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttachedIoInfo {
    /// Raw u16 device type ID from the wire (maps to `DeviceType`).
    pub device_type_id: u16,
    /// Firmware version string (decoded from int32LE at offset 7).
    pub hw_version: Version,
    /// Software version string.
    pub sw_version: Version,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualIoInfo {
    pub device_type_id: u16,
    pub first_port_id: u8,
    pub second_port_id: u8,
}

/// Decoded version (matches `decodeVersion` in utils.ts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub build: u16,
}

impl Version {
    /// Decode from a little-endian i32 on the wire.
    pub fn from_le_i32(raw: i32) -> Self {
        // decodeVersion pads to 8 hex digits, splits as [0],[1],[2..4],[4..]
        let hex = format!("{:08x}", raw as u32);
        let major = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0);
        let minor = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0);
        let patch = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let build = u16::from_str_radix(&hex[4..8], 16).unwrap_or(0);
        Self {
            major,
            minor,
            patch,
            build,
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{:02}.{:04}",
            self.major, self.minor, self.patch, self.build
        )
    }
}

/// Parsed `PORT_INPUT_FORMAT_SETUP_SINGLE` (0x41).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortInputFormatSetupSingle {
    pub port_id: u8,
    pub mode: u8,
    /// Delta interval (little-endian u32).
    pub delta: u32,
    /// `true` = subscribe, `false` = unsubscribe.
    pub enable_notifications: bool,
}

/// Parsed `PORT_VALUE_SINGLE` sensor notification (0x45).
#[derive(Debug, Clone, PartialEq)]
pub struct PortValueSingle {
    pub port_id: u8,
    /// Raw sensor bytes — parsed by the device layer.
    pub data: Bytes,
}

/// Parsed `PORT_INPUT_FORMAT_SINGLE` active-mode notification (0x47).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortInputFormatSingle {
    pub port_id: u8,
    pub mode: u8,
}

/// Parsed `PORT_OUTPUT_COMMAND` (0x81).
#[derive(Debug, Clone, PartialEq)]
pub struct PortOutputCommand {
    pub port_id: u8,
    pub startup_and_completion: u8,
    /// Subcommand + parameters — device layer interprets these.
    pub payload: Bytes,
}

/// Parsed `PORT_OUTPUT_COMMAND_FEEDBACK` (0x82).
///
/// A single 0x82 frame may carry feedback for multiple ports.
#[derive(Debug, Clone, PartialEq)]
pub struct PortOutputCommandFeedback {
    /// `(port_id, feedback_byte)` pairs — raw feedback byte maps to `CommandFeedback`.
    pub entries: Vec<(u8, CommandFeedback)>,
}

// ── Framing helper ────────────────────────────────────────────────────────────

/// Accumulation buffer for fragmented BLE notifications.
///
/// BLE MTU may deliver partial frames. Push incoming chunks with
/// [`FrameBuffer::push`]; it returns complete message frames as they become
/// available.
pub struct FrameBuffer {
    buf: BytesMut,
}

impl FrameBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buf: BytesMut::new(),
        }
    }

    /// Append `chunk` and drain any complete frames, returning them in order.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Bytes> {
        self.buf.extend_from_slice(chunk);
        std::iter::from_fn(|| {
            let len = *self.buf.first()? as usize;
            if len == 0 || self.buf.len() < len {
                return None;
            }
            Some(self.buf.split_to(len).freeze())
        })
        .collect()
    }
}

impl Default for FrameBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a single complete LPF2 frame.
///
/// `buf` must be exactly one frame (i.e. `buf.len() == buf[0]`). Use
/// [`FrameBuffer`] to accumulate chunks first.
///
/// # Errors
/// Returns [`Error::Parse`] if the frame is too short or a discriminant byte
/// does not map to a known enum variant.
pub fn parse(buf: &[u8]) -> Result<LpfMessage> {
    if buf.len() < 3 {
        return Err(Error::Parse(format!(
            "frame too short: {} bytes",
            buf.len()
        )));
    }
    let declared_len = buf[0] as usize;
    if buf.len() < declared_len {
        return Err(Error::Parse(format!(
            "frame truncated: declared {declared_len}, got {}",
            buf.len()
        )));
    }
    // buf[1] is reserved/hub-id, always 0x00 — we ignore it
    let msg_type_byte = buf[2];
    let payload = &buf[3..declared_len];

    match MessageType::try_from(msg_type_byte) {
        Ok(MessageType::HubProperties) => parse_hub_property(payload),
        Ok(MessageType::HubActions) => parse_hub_action(payload),
        Ok(MessageType::HubAlerts) => parse_hub_alert(payload),
        Ok(MessageType::HubAttachedIo) => parse_hub_attached_io(payload),
        Ok(MessageType::GenericErrorMessages) => parse_generic_error(payload),
        Ok(MessageType::PortInputFormatSetupSingle) => parse_port_input_format_setup(payload),
        Ok(MessageType::PortValueSingle) => parse_port_value_single(payload),
        Ok(MessageType::PortInputFormatSingle) => parse_port_input_format_single(payload),
        Ok(MessageType::PortOutputCommand) => parse_port_output_command(payload),
        Ok(MessageType::PortOutputCommandFeedback) => parse_port_output_feedback(payload),
        Ok(_) | Err(_) => Ok(LpfMessage::Unknown {
            message_type: msg_type_byte,
            payload: Bytes::copy_from_slice(payload),
        }),
    }
}

/// Encode a message for transmission to the hub.
///
/// Prepends the two-byte header `[total_len, 0x00]` as required by LPF2.
#[must_use]
pub fn encode(msg: &LpfMessage) -> Bytes {
    let inner = encode_inner(msg);
    let total = inner.len() + 2; // +2 for length byte + reserved byte
    Bytes::from([&[total as u8, 0x00][..], &inner].concat())
}

// ── Private parsers ───────────────────────────────────────────────────────────

fn parse_hub_property(payload: &[u8]) -> Result<LpfMessage> {
    if payload.len() < 2 {
        return Err(Error::Parse("HubProperty payload too short".into()));
    }
    let property = HubPropertyReference::try_from(payload[0])?;
    let operation = HubPropertyOperation::try_from(payload[1])?;
    Ok(LpfMessage::HubProperty(HubPropertyMessage {
        property,
        operation,
        payload: Bytes::copy_from_slice(&payload[2..]),
    }))
}

fn parse_hub_action(payload: &[u8]) -> Result<LpfMessage> {
    if payload.is_empty() {
        return Err(Error::Parse("HubAction payload empty".into()));
    }
    Ok(LpfMessage::HubAction(ActionType::try_from(payload[0])?))
}

fn parse_hub_alert(payload: &[u8]) -> Result<LpfMessage> {
    if payload.len() < 2 {
        return Err(Error::Parse("HubAlert payload too short".into()));
    }
    Ok(LpfMessage::HubAlert(HubAlertMessage {
        alert_type: AlertType::try_from(payload[0])?,
        payload: AlertPayload::try_from(payload[1])?,
    }))
}

fn parse_hub_attached_io(payload: &[u8]) -> Result<LpfMessage> {
    if payload.is_empty() {
        return Err(Error::Parse("HubAttachedIo payload empty".into()));
    }
    let port_id = payload[0];
    let event = IoEvent::try_from(payload[1])?;

    let (attached, virtual_attached) = match event {
        IoEvent::AttachedIo if payload.len() >= 12 => {
            let device_type_id = u16::from_le_bytes([payload[2], payload[3]]);
            let hw_raw = i32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
            let sw_raw = i32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
            (
                Some(AttachedIoInfo {
                    device_type_id,
                    hw_version: Version::from_le_i32(hw_raw),
                    sw_version: Version::from_le_i32(sw_raw),
                }),
                None,
            )
        }
        IoEvent::AttachedVirtualIo if payload.len() >= 6 => {
            let device_type_id = u16::from_le_bytes([payload[2], payload[3]]);
            (
                None,
                Some(VirtualIoInfo {
                    device_type_id,
                    first_port_id: payload[4],
                    second_port_id: payload[5],
                }),
            )
        }
        IoEvent::DetachedIo | _ => (None, None),
    };

    Ok(LpfMessage::HubAttachedIo(HubAttachedIoMessage {
        port_id,
        event,
        attached,
        virtual_attached,
    }))
}

fn parse_generic_error(payload: &[u8]) -> Result<LpfMessage> {
    if payload.len() < 2 {
        return Err(Error::Parse("GenericError payload too short".into()));
    }
    Ok(LpfMessage::GenericError {
        command: payload[0],
        code: payload[1],
    })
}

fn parse_port_input_format_setup(payload: &[u8]) -> Result<LpfMessage> {
    if payload.len() < 7 {
        return Err(Error::Parse(
            "PortInputFormatSetup payload too short".into(),
        ));
    }
    Ok(LpfMessage::PortInputFormatSetupSingle(
        PortInputFormatSetupSingle {
            port_id: payload[0],
            mode: payload[1],
            delta: u32::from_le_bytes([payload[2], payload[3], payload[4], payload[5]]),
            enable_notifications: payload[6] != 0,
        },
    ))
}

fn parse_port_value_single(payload: &[u8]) -> Result<LpfMessage> {
    if payload.is_empty() {
        return Err(Error::Parse("PortValueSingle payload empty".into()));
    }
    Ok(LpfMessage::PortValueSingle(PortValueSingle {
        port_id: payload[0],
        data: Bytes::copy_from_slice(&payload[1..]),
    }))
}

fn parse_port_input_format_single(payload: &[u8]) -> Result<LpfMessage> {
    if payload.len() < 2 {
        return Err(Error::Parse(
            "PortInputFormatSingle payload too short".into(),
        ));
    }
    Ok(LpfMessage::PortInputFormatSingle(PortInputFormatSingle {
        port_id: payload[0],
        mode: payload[1],
    }))
}

fn parse_port_output_command(payload: &[u8]) -> Result<LpfMessage> {
    if payload.len() < 2 {
        return Err(Error::Parse("PortOutputCommand payload too short".into()));
    }
    Ok(LpfMessage::PortOutputCommand(PortOutputCommand {
        port_id: payload[0],
        startup_and_completion: payload[1],
        payload: Bytes::copy_from_slice(&payload[2..]),
    }))
}

fn parse_port_output_feedback(payload: &[u8]) -> Result<LpfMessage> {
    // Each feedback entry is 2 bytes: port_id + feedback_byte
    if payload.len() < 2 || payload.len() % 2 != 0 {
        return Err(Error::Parse(format!(
            "PortOutputCommandFeedback unexpected payload length {}",
            payload.len()
        )));
    }
    let entries = payload
        .chunks_exact(2)
        .map(|c| CommandFeedback::try_from(c[1]).map(|fb| (c[0], fb)))
        .collect::<Result<Vec<_>>>()?;
    Ok(LpfMessage::PortOutputCommandFeedback(
        PortOutputCommandFeedback { entries },
    ))
}

// ── Encoder helpers ───────────────────────────────────────────────────────────

fn encode_inner(msg: &LpfMessage) -> Bytes {
    match msg {
        LpfMessage::HubProperty(m) => Bytes::from(
            [
                &[
                    MessageType::HubProperties as u8,
                    m.property as u8,
                    m.operation as u8,
                ][..],
                &m.payload,
            ]
            .concat(),
        ),
        LpfMessage::HubAction(a) => Bytes::from(vec![MessageType::HubActions as u8, *a as u8]),
        LpfMessage::PortInputFormatSetupSingle(m) => Bytes::from(
            [
                &[
                    MessageType::PortInputFormatSetupSingle as u8,
                    m.port_id,
                    m.mode,
                ][..],
                &m.delta.to_le_bytes(),
                &[u8::from(m.enable_notifications)],
            ]
            .concat(),
        ),
        LpfMessage::PortOutputCommand(m) => Bytes::from(
            [
                &[
                    MessageType::PortOutputCommand as u8,
                    m.port_id,
                    m.startup_and_completion,
                ][..],
                &m.payload,
            ]
            .concat(),
        ),
        // Upstream-only messages are not normally encoded; fall back to Unknown.
        LpfMessage::Unknown {
            message_type,
            payload,
        } => Bytes::from([&[*message_type][..], payload.as_ref()].concat()),
        _ => {
            // Not encodable by the host — return empty body; header is still prepended.
            Bytes::new()
        }
    }
}

// (CollectBytes helper removed — no longer needed)

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::consts::{
        ActionType, AlertPayload, AlertType, CommandFeedback, HubPropertyOperation,
        HubPropertyReference, IoEvent,
    };

    // Helper: build a complete LPF2 frame from type + payload bytes.
    fn frame(msg_type: u8, payload: &[u8]) -> Vec<u8> {
        let len = 3 + payload.len();
        let mut f = vec![len as u8, 0x00, msg_type];
        f.extend_from_slice(payload);
        f
    }

    // ── FrameBuffer ───────────────────────────────────────────────────────────

    #[test]
    fn frame_buffer_single_chunk() {
        let mut fb = FrameBuffer::new();
        let data = frame(
            0x04,
            &[0x00, 0x01, 0x27, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        );
        let frames = fb.push(&data);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].as_ref(), data.as_slice());
    }

    #[test]
    fn frame_buffer_two_frames_in_one_chunk() {
        let mut fb = FrameBuffer::new();
        let mut data = frame(0x02, &[0x01]); // HubAction
        data.extend(frame(0x02, &[0x02])); // another HubAction
        let frames = fb.push(&data);
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn frame_buffer_fragmented() {
        let mut fb = FrameBuffer::new();
        let data = frame(0x02, &[0x01]);
        let (first, second) = data.split_at(2);
        assert_eq!(fb.push(first).len(), 0);
        assert_eq!(fb.push(second).len(), 1);
    }

    // ── parse ─────────────────────────────────────────────────────────────────

    #[test]
    fn parse_hub_action() {
        // HUB_ACTIONS disconnect: [0x04, 0x00, 0x02, 0x02]
        let data = frame(
            MessageType::HubActions as u8,
            &[ActionType::Disconnect as u8],
        );
        let msg = parse(&data).unwrap();
        assert_eq!(msg, LpfMessage::HubAction(ActionType::Disconnect));
    }

    #[test]
    fn parse_hub_alert() {
        let data = frame(
            MessageType::HubAlerts as u8,
            &[AlertType::LowVoltage as u8, AlertPayload::Alert as u8],
        );
        let msg = parse(&data).unwrap();
        assert_eq!(
            msg,
            LpfMessage::HubAlert(HubAlertMessage {
                alert_type: AlertType::LowVoltage,
                payload: AlertPayload::Alert,
            })
        );
    }

    #[test]
    fn parse_hub_attached_io_attached() {
        // Real wire frame: port 0x00, ATTACHED, device 0x0027 (MediumLinearMotor=38=0x26... actually 0x27 in older firmwares), hw/sw versions
        #[rustfmt::skip]
        let payload = &[
            0x00,               // port_id
            IoEvent::AttachedIo as u8,
            0x26, 0x00,         // device_type_id = 0x0026 = 38 = MediumLinearMotor
            0x00, 0x00, 0x00, 0x10, // hw_version le_i32
            0x00, 0x00, 0x00, 0x10, // sw_version le_i32
        ];
        let data = frame(MessageType::HubAttachedIo as u8, payload);
        let msg = parse(&data).unwrap();
        if let LpfMessage::HubAttachedIo(m) = msg {
            assert_eq!(m.port_id, 0x00);
            assert_eq!(m.event, IoEvent::AttachedIo);
            let info = m.attached.unwrap();
            assert_eq!(info.device_type_id, 0x0026);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_hub_attached_io_detached() {
        let payload = &[0x01, IoEvent::DetachedIo as u8];
        let data = frame(MessageType::HubAttachedIo as u8, payload);
        let msg = parse(&data).unwrap();
        if let LpfMessage::HubAttachedIo(m) = msg {
            assert_eq!(m.port_id, 0x01);
            assert_eq!(m.event, IoEvent::DetachedIo);
            assert!(m.attached.is_none());
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_port_value_single() {
        let payload = &[0x00, 0x64]; // port 0, data = [100]
        let data = frame(MessageType::PortValueSingle as u8, payload);
        let msg = parse(&data).unwrap();
        if let LpfMessage::PortValueSingle(v) = msg {
            assert_eq!(v.port_id, 0x00);
            assert_eq!(v.data.as_ref(), &[0x64u8]);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_feedback_single_entry() {
        let payload = &[0x00, CommandFeedback::ExecutionCompleted as u8];
        let data = frame(MessageType::PortOutputCommandFeedback as u8, payload);
        let msg = parse(&data).unwrap();
        if let LpfMessage::PortOutputCommandFeedback(f) = msg {
            assert_eq!(f.entries, vec![(0x00, CommandFeedback::ExecutionCompleted)]);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_feedback_two_entries() {
        let payload = &[
            0x00,
            CommandFeedback::ExecutionCompleted as u8,
            0x01,
            CommandFeedback::ExecutionBusy as u8,
        ];
        let data = frame(MessageType::PortOutputCommandFeedback as u8, payload);
        let msg = parse(&data).unwrap();
        if let LpfMessage::PortOutputCommandFeedback(f) = msg {
            assert_eq!(f.entries.len(), 2);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_unknown_message_type_does_not_error() {
        let data = frame(0xFE, &[0x01, 0x02]);
        let msg = parse(&data).unwrap();
        assert!(matches!(msg, LpfMessage::Unknown { .. }));
    }

    // ── encode ────────────────────────────────────────────────────────────────

    #[test]
    fn encode_hub_action_shutdown() {
        // TS source: Buffer.from([0x02, 0x01]) → after send() wrapping = [0x04, 0x00, 0x02, 0x01]
        let msg = LpfMessage::HubAction(ActionType::SwitchOffHub);
        let bytes = encode(&msg);
        assert_eq!(bytes.as_ref(), &[0x04, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn encode_port_input_format_setup_subscribe() {
        // TS: Buffer.from([0x41, portId, mode, 0x01, 0x00, 0x00, 0x00, 0x01])
        //   after send() header: [0x0a, 0x00, 0x41, ...]
        let msg = LpfMessage::PortInputFormatSetupSingle(PortInputFormatSetupSingle {
            port_id: 0x00,
            mode: 0x00,
            delta: 1,
            enable_notifications: true,
        });
        let bytes = encode(&msg);
        assert_eq!(
            bytes.as_ref(),
            &[0x0a, 0x00, 0x41, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01]
        );
    }

    #[test]
    fn encode_port_input_format_setup_unsubscribe() {
        // TS: Buffer.from([0x41, portId, mode, 0x01, 0x00, 0x00, 0x00, 0x00])
        let msg = LpfMessage::PortInputFormatSetupSingle(PortInputFormatSetupSingle {
            port_id: 0x01,
            mode: 0x02,
            delta: 1,
            enable_notifications: false,
        });
        let bytes = encode(&msg);
        assert_eq!(
            bytes.as_ref(),
            &[0x0a, 0x00, 0x41, 0x01, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn version_display() {
        let v = Version::from_le_i32(0x1000_0000_u32 as i32);
        assert_eq!(v.to_string(), "1.0.00.0000");
    }

    #[test]
    fn hub_property_round_trip() {
        // Request FW version: [0x05, 0x00, 0x01, 0x03, 0x05]
        let property_payload = Bytes::new();
        let msg = LpfMessage::HubProperty(HubPropertyMessage {
            property: HubPropertyReference::FwVersion,
            operation: HubPropertyOperation::RequestUpdateDownstream,
            payload: property_payload,
        });
        let bytes = encode(&msg);
        // [total=5, 0x00, type=0x01, ref=0x03, op=0x05]
        assert_eq!(bytes.as_ref(), &[0x05, 0x00, 0x01, 0x03, 0x05]);

        // Parse it back
        let parsed = parse(&bytes).unwrap();
        if let LpfMessage::HubProperty(m) = parsed {
            assert_eq!(m.property, HubPropertyReference::FwVersion);
            assert_eq!(m.operation, HubPropertyOperation::RequestUpdateDownstream);
        } else {
            panic!("wrong variant");
        }
    }
}
