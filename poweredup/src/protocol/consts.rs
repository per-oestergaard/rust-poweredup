//! Protocol constants ported from `node-poweredup/src/consts.ts`.
//!
//! Every integer constant is a `#[repr(u8)]` (or `#[repr(u16)]`) enum with a
//! derived `TryFrom` implementation so raw bytes from the BLE codec layer can
//! be converted safely without panicking.

use crate::error::{Error, Result};

// ── Helper macro ────────────────────────────────────────────────────────────

/// Declares a C-like enum and implements `TryFrom<$repr>` with parse-error on
/// unknown values. All variants must have explicit discriminants.
macro_rules! enum_try_from {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident : $repr:ty {
            $( $variant:ident = $value:expr ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[repr($repr)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis enum $name {
            $( $variant = $value, )*
        }

        impl TryFrom<$repr> for $name {
            type Error = Error;
            fn try_from(v: $repr) -> Result<Self> {
                match v {
                    $( $value => Ok(Self::$variant), )*
                    _ => Err(Error::Parse(format!(
                        "unknown {} discriminant: {:#x}", stringify!($name), v
                    ))),
                }
            }
        }
    };
}

// ── Hub types ────────────────────────────────────────────────────────────────

enum_try_from! {
    /// Which physical hub model is connected.
    pub enum HubType : u8 {
        Unknown         = 0,
        WeDo2SmartHub   = 1,
        MoveHub         = 2,
        Hub             = 3,
        RemoteControl   = 4,
        DuploTrainBase  = 5,
        TechnicMediumHub = 6,
        Mario           = 7,
        TechnicSmallHub = 8,
        Luigi           = 9,
        Peach           = 10,
    }
}

// ── Device types ─────────────────────────────────────────────────────────────

enum_try_from! {
    /// Peripheral device type reported in `HUB_ATTACHED_IO` messages.
    pub enum DeviceType : u8 {
        Unknown                         = 0,
        SimpleMediumLinearMotor         = 1,
        TrainMotor                      = 2,
        Light                           = 8,
        VoltageSensor                   = 20,
        CurrentSensor                   = 21,
        PiezoBuzzer                     = 22,
        HubLed                          = 23,
        TiltSensor                      = 34,
        MotionSensor                    = 35,
        ColorDistanceSensor             = 37,
        MediumLinearMotor               = 38,
        MoveHubMediumLinearMotor        = 39,
        MoveHubTiltSensor               = 40,
        DuploTrainBaseMotor             = 41,
        DuploTrainBaseSpeaker           = 42,
        DuploTrainBaseColorSensor       = 43,
        DuploTrainBaseSpeedometer       = 44,
        TechnicLargeLinearMotor         = 46,  // Technic Control+
        TechnicXLargeLinearMotor        = 47,  // Technic Control+
        TechnicMediumAngularMotor       = 48,  // Spike Prime
        TechnicLargeAngularMotor        = 49,  // Spike Prime
        TechnicMediumHubGestSensor      = 54,
        RemoteControlButton             = 55,
        RemoteControlRssi               = 56,
        TechnicMediumHubAccelerometer   = 57,
        TechnicMediumHubGyroSensor      = 58,
        TechnicMediumHubTiltSensor      = 59,
        TechnicMediumHubTemperatureSensor = 60,
        TechnicColorSensor              = 61,  // Spike Prime
        TechnicDistanceSensor           = 62,  // Spike Prime
        TechnicForceSensor              = 63,  // Spike Prime
        Technic3x3ColorLightMatrix      = 64,  // Spike Essential
        TechnicSmallAngularMotor        = 65,  // Spike Essential
        MarioAccelerometer              = 71,
        MarioBarcodeReader              = 73,
        MarioPantsSensor                = 74,
        TechnicMediumAngularMotorGrey   = 75,  // Mindstorms
        TechnicLargeAngularMotorGrey    = 76,  // Technic Control+
    }
}

// ── Color ────────────────────────────────────────────────────────────────────

enum_try_from! {
    pub enum Color : u8 {
        Black     = 0,
        Pink      = 1,
        Purple    = 2,
        Blue      = 3,
        LightBlue = 4,
        Cyan      = 5,
        Green     = 6,
        Yellow    = 7,
        Orange    = 8,
        Red       = 9,
        White     = 10,
        None      = 255,
    }
}

// ── Button state ─────────────────────────────────────────────────────────────

enum_try_from! {
    pub enum ButtonState : u8 {
        Released = 0,
        Up       = 1,
        Pressed  = 2,
        Stop     = 127,
        Down     = 255,
    }
}

// ── Motor braking style ───────────────────────────────────────────────────────

enum_try_from! {
    /// End-state behaviour when a motor command completes.
    pub enum BrakingStyle : u8 {
        Float = 0,
        Hold  = 126,
        Brake = 127,
    }
}

// ── Duplo train sound ─────────────────────────────────────────────────────────

enum_try_from! {
    pub enum DuploTrainBaseSound : u8 {
        Brake            = 3,
        StationDeparture = 5,
        WaterRefill      = 7,
        Horn             = 9,
        Steam            = 10,
    }
}

// ── BLE manufacturer data IDs ─────────────────────────────────────────────────

enum_try_from! {
    /// Byte 3 of the BLE advertisement manufacturer data identifies the hub model.
    pub enum BleManufacturerData : u8 {
        DuploTrainBaseId  = 32,
        MoveHubId         = 64,
        HubId             = 65,
        RemoteControlId   = 66,
        MarioId           = 67,
        LuigiId           = 68,
        PeachId           = 69,
        TechnicMediumHubId = 128,
        TechnicSmallHubId  = 131,
    }
}

// ── BLE UUIDs ─────────────────────────────────────────────────────────────────

/// BLE service and characteristic UUIDs used by this library.
pub mod ble_uuid {
    // WeDo 2.0 services
    pub const WEDO2_SERVICE: &str = "00001523-1212-efde-1523-785feabcd123";
    pub const WEDO2_SERVICE_2: &str = "00004f0e-1212-efde-1523-785feabcd123";
    // LPF2 (Powered UP, Technic, etc.)
    pub const LPF2_SERVICE: &str = "00001623-1212-efde-1623-785feabcd123";
    pub const LPF2_ALL: &str = "00001624-1212-efde-1623-785feabcd123";
    // WeDo 2.0 characteristics
    pub const WEDO2_BUTTON: &str = "00001526-1212-efde-1523-785feabcd123";
    pub const WEDO2_PORT_TYPE: &str = "00001527-1212-efde-1523-785feabcd123";
    pub const WEDO2_SENSOR_VALUE: &str = "00001560-1212-efde-1523-785feabcd123";
    pub const WEDO2_MOTOR_WRITE: &str = "00001565-1212-efde-1523-785feabcd123";
}

// ── Message types ─────────────────────────────────────────────────────────────

enum_try_from! {
    /// LPF2 upstream / downstream message type byte.
    /// See <https://lego.github.io/lego-ble-wireless-protocol-docs/index.html#message-types>
    pub enum MessageType : u8 {
        HubProperties                   = 0x01,
        HubActions                      = 0x02,
        HubAlerts                       = 0x03,
        HubAttachedIo                   = 0x04,
        GenericErrorMessages            = 0x05,
        HwNetworkCommands               = 0x08,
        FwUpdateGoIntoBootMode          = 0x10,
        FwUpdateLockMemory              = 0x11,
        FwUpdateLockStatusRequest       = 0x12,
        FwLockStatus                    = 0x13,
        PortInformationRequest          = 0x21,
        PortModeInformationRequest      = 0x22,
        PortInputFormatSetupSingle      = 0x41,
        PortInputFormatSetupCombinedMode = 0x42,
        PortInformation                 = 0x43,
        PortModeInformation             = 0x44,
        PortValueSingle                 = 0x45,
        PortValueCombinedMode           = 0x46,
        PortInputFormatSingle           = 0x47,
        PortInputFormatCombinedMode     = 0x48,
        VirtualPortSetup                = 0x61,
        PortOutputCommand               = 0x81,
        PortOutputCommandFeedback       = 0x82,
    }
}

// ── Hub property reference ────────────────────────────────────────────────────

enum_try_from! {
    pub enum HubPropertyReference : u8 {
        AdvertisingName          = 0x01,
        Button                   = 0x02,
        FwVersion                = 0x03,
        HwVersion                = 0x04,
        Rssi                     = 0x05,
        BatteryVoltage           = 0x06,
        BatteryType              = 0x07,
        ManufacturerName         = 0x08,
        RadioFirmwareVersion     = 0x09,
        LegoWirelessProtocolVersion = 0x0A,
        SystemTypeId             = 0x0B,
        HwNetworkId              = 0x0C,
        PrimaryMacAddress        = 0x0D,
        SecondaryMacAddress      = 0x0E,
        HardwareNetworkFamily    = 0x0F,
    }
}

// ── Hub property operation ────────────────────────────────────────────────────

enum_try_from! {
    pub enum HubPropertyOperation : u8 {
        SetDownstream             = 0x01,
        EnableUpdatesDownstream   = 0x02,
        DisableUpdatesDownstream  = 0x03,
        ResetDownstream           = 0x04,
        RequestUpdateDownstream   = 0x05,
        UpdateUpstream            = 0x06,
    }
}

// ── Hub action type ───────────────────────────────────────────────────────────

enum_try_from! {
    /// Actions sent downstream to the hub or received upstream as notifications.
    /// See <https://lego.github.io/lego-ble-wireless-protocol-docs/index.html#action-types>
    pub enum ActionType : u8 {
        SwitchOffHub           = 0x01,
        Disconnect             = 0x02,
        VccPortControlOn       = 0x03,
        VccPortControlOff      = 0x04,
        ActivateBusyIndication = 0x05,
        ResetBusyIndication    = 0x06,
        Shutdown               = 0x2F,
        HubWillSwitchOff       = 0x30,
        HubWillDisconnect      = 0x31,
        HubWillGoIntoBootMode  = 0x32,
    }
}

// ── Alert type / payload ──────────────────────────────────────────────────────

enum_try_from! {
    pub enum AlertType : u8 {
        LowVoltage          = 0x01,
        HighCurrent         = 0x02,
        LowSignalStrength   = 0x03,
        OverPowerCondition  = 0x04,
    }
}

enum_try_from! {
    pub enum AlertPayload : u8 {
        StatusOk = 0x00,
        Alert    = 0xFF,
    }
}

// ── IO attachment event ───────────────────────────────────────────────────────

enum_try_from! {
    pub enum IoEvent : u8 {
        DetachedIo        = 0x00,
        AttachedIo        = 0x01,
        AttachedVirtualIo = 0x02,
    }
}

// ── Error codes ───────────────────────────────────────────────────────────────

enum_try_from! {
    /// See <https://lego.github.io/lego-ble-wireless-protocol-docs/index.html#error-codes>
    pub enum ErrorCode : u8 {
        Ack                  = 0x01,
        Mack                 = 0x02,
        BufferOverflow       = 0x03,
        Timeout              = 0x04,
        CommandNotRecognized = 0x05,
        InvalidUse           = 0x06,
        Overcurrent          = 0x07,
        InternalError        = 0x08,
    }
}

// ── Command feedback ─────────────────────────────────────────────────────────

enum_try_from! {
    /// State machine values for port-output command tracking.
    pub enum CommandFeedback : u8 {
        TransmissionPending   = 0x00,
        TransmissionBusy      = 0x10,
        ExecutionPending      = 0x20,
        ExecutionBusy         = 0x21,
        ExecutionCompleted    = 0x22,
        ExecutionDiscarded    = 0x24,
        FeedbackDisabled      = 0x26,
        TransmissionDiscarded = 0x44,
        FeedbackMissing       = 0x66,
    }
}

// ── Mode information type ─────────────────────────────────────────────────────

enum_try_from! {
    pub enum ModeInformationType : u8 {
        Name           = 0x00,
        Raw            = 0x01,
        Pct            = 0x02,
        Si             = 0x03,
        Symbol         = 0x04,
        Mapping        = 0x05,
        UsedInternally = 0x06,
        MotorBias      = 0x07,
        CapabilityBits = 0x08,
        ValueFormat    = 0x80,
    }
}

// ── Tilt direction ────────────────────────────────────────────────────────────

enum_try_from! {
    pub enum TiltDirection : u8 {
        Neutral  = 0,
        Backward = 3,
        Right    = 5,
        Left     = 7,
        Forward  = 9,
        Unknown  = 10,
    }
}

// ── Mario pants type ──────────────────────────────────────────────────────────

enum_try_from! {
    pub enum MarioPantsType : u8 {
        None       = 0x00,
        Propeller  = 0x06,
        Cat        = 0x11,
        Fire       = 0x12,
        Normal     = 0x21,
        Builder    = 0x22,
    }
}

// ── Mario color (u16 wire value) ──────────────────────────────────────────────

enum_try_from! {
    pub enum MarioColor : u16 {
        White  = 0x1300,
        Red    = 0x1500,
        Blue   = 0x1700,
        Yellow = 0x1800,
        Black  = 0x1a00,
        Green  = 0x2500,
        Brown  = 0x6a00,
        Cyan   = 0x4201,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_type_round_trip() {
        for disc in 0u8..=10 {
            let ht = HubType::try_from(disc).expect("should parse");
            assert_eq!(ht as u8, disc);
        }
    }

    #[test]
    fn hub_type_unknown_wire_value_is_error() {
        assert!(HubType::try_from(99).is_err());
    }

    #[test]
    fn device_type_known_values() {
        assert_eq!(
            DeviceType::try_from(38).unwrap(),
            DeviceType::MediumLinearMotor
        );
        assert_eq!(
            DeviceType::try_from(49).unwrap(),
            DeviceType::TechnicLargeAngularMotor
        );
        assert_eq!(
            DeviceType::try_from(255).unwrap_err().to_string(),
            "Parse error: unknown DeviceType discriminant: 0xff"
        );
    }

    #[test]
    fn message_type_round_trip() {
        let known: &[(u8, MessageType)] = &[
            (0x01, MessageType::HubProperties),
            (0x04, MessageType::HubAttachedIo),
            (0x81, MessageType::PortOutputCommand),
            (0x82, MessageType::PortOutputCommandFeedback),
        ];
        for &(byte, expected) in known {
            assert_eq!(MessageType::try_from(byte).unwrap(), expected);
            assert_eq!(expected as u8, byte);
        }
    }

    #[test]
    fn command_feedback_all_variants() {
        let cases: &[(u8, CommandFeedback)] = &[
            (0x00, CommandFeedback::TransmissionPending),
            (0x10, CommandFeedback::TransmissionBusy),
            (0x20, CommandFeedback::ExecutionPending),
            (0x21, CommandFeedback::ExecutionBusy),
            (0x22, CommandFeedback::ExecutionCompleted),
            (0x24, CommandFeedback::ExecutionDiscarded),
            (0x26, CommandFeedback::FeedbackDisabled),
            (0x44, CommandFeedback::TransmissionDiscarded),
            (0x66, CommandFeedback::FeedbackMissing),
        ];
        for &(byte, expected) in cases {
            assert_eq!(CommandFeedback::try_from(byte).unwrap(), expected);
        }
    }

    #[test]
    fn braking_style_values() {
        assert_eq!(BrakingStyle::try_from(0).unwrap(), BrakingStyle::Float);
        assert_eq!(BrakingStyle::try_from(126).unwrap(), BrakingStyle::Hold);
        assert_eq!(BrakingStyle::try_from(127).unwrap(), BrakingStyle::Brake);
    }

    #[test]
    fn color_none_is_255() {
        assert_eq!(Color::try_from(255).unwrap(), Color::None);
    }

    #[test]
    fn mario_color_u16() {
        assert_eq!(MarioColor::try_from(0x1300u16).unwrap(), MarioColor::White);
        assert!(MarioColor::try_from(0x0000u16).is_err());
    }
}
