//! Device trait and factory — the abstraction over all physical sensors/actuators.
//!
//! Raw `PORT_VALUE_SINGLE` notifications are dispatched to the owning [`Device`] via
//! [`Device::receive`]. `PORT_INPUT_FORMAT_SINGLE` mode updates arrive via
//! [`Device::set_mode`]. Typed [`Event`]s are returned.

use bytes::Bytes;

use crate::{error::Result, protocol::consts::DeviceType};

pub mod light;
pub mod motor;
pub mod sensor;

// ── Event ─────────────────────────────────────────────────────────────────────

/// A typed event produced by a device after parsing a raw port-value notification.
#[derive(Debug, Clone)]
pub enum Event {
    /// Tacho/absolute motor rotation encoder value.
    MotorRotate { port_id: u8, degrees: i32 },
    /// Absolute motor position in degrees.
    MotorAngle { port_id: u8, angle: i16 },
    /// Detected color.
    Color { port_id: u8, color: crate::protocol::consts::Color },
    /// Distance measurement in millimetres.
    Distance { port_id: u8, distance_mm: u32 },
    /// Combined color + distance.
    ColorAndDistance {
        port_id: u8,
        color: crate::protocol::consts::Color,
        distance_mm: u32,
    },
    /// Reflectivity percentage (0–100).
    Reflect { port_id: u8, percent: u8 },
    /// Ambient light percentage (0–100).
    Ambient { port_id: u8, percent: u8 },
    /// 2-axis (or 3-axis) tilt in degrees.
    Tilt { port_id: u8, x: i8, y: i8, z: Option<i8> },
    /// Hub voltage in millivolts.
    Voltage { port_id: u8, millivolts: u32 },
    /// Hub current in milliamps.
    Current { port_id: u8, milliamps: u32 },
    /// Remote-control button raw state byte.
    RemoteButton { port_id: u8, state: u8 },
    /// Raw passthrough for not-yet-decoded device types.
    Raw { port_id: u8, mode: u8, data: Bytes },
}

// ── Device trait ──────────────────────────────────────────────────────────────

/// A sensor or actuator attached to a hub port.
pub trait Device: Send + 'static {
    /// Raw u16 device type ID from the wire (`HUB_ATTACHED_IO` message).
    fn device_type_id(&self) -> u16;

    /// Port the device is attached to.
    fn port_id(&self) -> u8;

    /// Update the active sensor mode (from `PORT_INPUT_FORMAT_SINGLE`).
    ///
    /// Devices that vary decode logic by mode override this.
    fn set_mode(&mut self, _mode: u8) {}

    /// Process an incoming `PORT_VALUE_SINGLE` notification.
    ///
    /// Returns `Some(event)` when the payload produces a meaningful typed event,
    /// `None` when it is silently consumed.
    ///
    /// # Errors
    /// Returns an error if the raw payload cannot be decoded into a typed value.
    fn receive(&mut self, data: &[u8]) -> Result<Option<Event>>;
}

// ── DeviceFactory ─────────────────────────────────────────────────────────────

/// Constructs the appropriate [`Device`] for a given device-type ID.
///
/// Unknown / not-yet-implemented types fall back to [`GenericDevice`].
pub struct DeviceFactory;

impl DeviceFactory {
    #[must_use]
    pub fn create(device_type_id: u16, port_id: u8) -> Box<dyn Device> {
        use light::{ColorLightMatrix, HubLed, Light, PiezoBuzzer};
        use motor::{AbsoluteMotorDevice, BasicMotorDevice, TachoMotorDevice};
        // device_type_id > 255 maps to no known DeviceType; handle gracefully
        let Ok(u8_id) = u8::try_from(device_type_id) else {
            return Box::new(GenericDevice {
                device_type_id,
                port_id,
                mode: 0,
            });
        };
        match DeviceType::try_from(u8_id) {
            // ── Basic motors (power only) ─────────────────────────────────────
            Ok(
                DeviceType::SimpleMediumLinearMotor
                | DeviceType::TrainMotor
                | DeviceType::DuploTrainBaseMotor,
            ) => Box::new(BasicMotorDevice::new(port_id, device_type_id)),
            // ── Tacho motors (speed + encoder) ────────────────────────────────
            Ok(
                DeviceType::MediumLinearMotor
                | DeviceType::MoveHubMediumLinearMotor
                | DeviceType::TechnicLargeLinearMotor
                | DeviceType::TechnicXLargeLinearMotor,
            ) => Box::new(TachoMotorDevice::new(port_id, device_type_id)),
            // ── Absolute motors (position + encoder) ──────────────────────────
            Ok(
                DeviceType::TechnicMediumAngularMotor
                | DeviceType::TechnicLargeAngularMotor
                | DeviceType::TechnicSmallAngularMotor
                | DeviceType::TechnicMediumAngularMotorGrey
                | DeviceType::TechnicLargeAngularMotorGrey,
            ) => Box::new(AbsoluteMotorDevice::new(port_id, device_type_id)),
            // ── Sensors ────────────────────────────────────────────────────────
            Ok(DeviceType::ColorDistanceSensor) => {
                Box::new(sensor::ColorDistanceSensor::new(port_id))
            }
            Ok(DeviceType::MotionSensor) => Box::new(sensor::MotionSensor::new(port_id)),
            Ok(DeviceType::TiltSensor | DeviceType::MoveHubTiltSensor) => {
                Box::new(sensor::TiltSensor::new(port_id))
            }
            Ok(DeviceType::VoltageSensor) => Box::new(sensor::VoltageSensor::new(port_id)),
            Ok(DeviceType::CurrentSensor) => Box::new(sensor::CurrentSensor::new(port_id)),
            Ok(DeviceType::RemoteControlButton) => {
                Box::new(sensor::RemoteControlButton::new(port_id))
            }
            // ── Lights and audio ──────────────────────────────────────────────
            Ok(DeviceType::HubLed) => Box::new(HubLed::new(port_id)),
            Ok(DeviceType::Light) => Box::new(Light::new(port_id)),
            Ok(DeviceType::PiezoBuzzer) => Box::new(PiezoBuzzer::new(port_id)),
            Ok(DeviceType::Technic3x3ColorLightMatrix) => {
                Box::new(ColorLightMatrix::new(port_id))
            }
            _ => Box::new(GenericDevice {
                device_type_id,
                port_id,
                mode: 0,
            }),
        }
    }
}

// ── GenericDevice ─────────────────────────────────────────────────────────────

struct GenericDevice {
    device_type_id: u16,
    port_id: u8,
    mode: u8,
}

impl Device for GenericDevice {
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
        Ok(Some(Event::Raw {
            port_id: self.port_id,
            mode: self.mode,
            data: Bytes::copy_from_slice(data),
        }))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_returns_generic_for_unknown_type() {
        let dev = DeviceFactory::create(0xFFFF, 3);
        assert_eq!(dev.device_type_id(), 0xFFFF);
        assert_eq!(dev.port_id(), 3);
    }

    #[test]
    fn generic_device_receive_returns_raw_event() {
        let mut dev = DeviceFactory::create(0xFFFF, 0);
        dev.set_mode(2);
        let result = dev.receive(&[0xAB, 0xCD]).unwrap();
        if let Some(Event::Raw {
            port_id,
            mode,
            data,
        }) = result
        {
            assert_eq!(port_id, 0);
            assert_eq!(mode, 2);
            assert_eq!(data.as_ref(), &[0xAB, 0xCD]);
        } else {
            panic!("expected Event::Raw");
        }
    }

    #[test]
    fn factory_dispatches_basic_motor() {
        let dev = DeviceFactory::create(2, 0); // TrainMotor
        assert_eq!(dev.device_type_id(), 2);
    }

    #[test]
    fn factory_dispatches_tacho_motor() {
        let dev = DeviceFactory::create(38, 1); // MediumLinearMotor
        assert_eq!(dev.device_type_id(), 38);
    }

    #[test]
    fn factory_dispatches_absolute_motor() {
        let dev = DeviceFactory::create(48, 2); // TechnicMediumAngularMotor
        assert_eq!(dev.device_type_id(), 48);
    }
}
