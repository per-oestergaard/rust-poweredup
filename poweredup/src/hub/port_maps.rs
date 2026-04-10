//! Per-hub-model port name → port ID mappings.
//!
//! Each function returns a fresh `HashMap` that the caller passes to
//! [`Hub::new`](super::Hub::new) (or via the typed constructors on
//! `Hub<Disconnected, T>`).

use std::collections::HashMap;

/// Port map for the **LEGO Powered UP Hub** (2-port hub, set #88009).
///
/// | Port name       | ID |
/// |-----------------|----|
/// | A               |  0 |
/// | B               |  1 |
/// | HUB_LED         | 50 |
/// | CURRENT_SENSOR  | 59 |
/// | VOLTAGE_SENSOR  | 60 |
#[must_use]
pub fn powered_up_hub() -> HashMap<String, u8> {
    HashMap::from([
        ("A".to_owned(), 0),
        ("B".to_owned(), 1),
        ("HUB_LED".to_owned(), 50),
        ("CURRENT_SENSOR".to_owned(), 59),
        ("VOLTAGE_SENSOR".to_owned(), 60),
    ])
}

/// Port map for the **LEGO BOOST Move Hub** (set #17101).
///
/// | Port name       | ID |
/// |-----------------|----|
/// | A               |  0 |
/// | B               |  1 |
/// | C               |  2 |
/// | D               |  3 |
/// | HUB_LED         | 50 |
/// | TILT_SENSOR     | 58 |
/// | CURRENT_SENSOR  | 59 |
/// | VOLTAGE_SENSOR  | 60 |
#[must_use]
pub fn move_hub() -> HashMap<String, u8> {
    HashMap::from([
        ("A".to_owned(), 0),
        ("B".to_owned(), 1),
        ("C".to_owned(), 2),
        ("D".to_owned(), 3),
        ("HUB_LED".to_owned(), 50),
        ("TILT_SENSOR".to_owned(), 58),
        ("CURRENT_SENSOR".to_owned(), 59),
        ("VOLTAGE_SENSOR".to_owned(), 60),
    ])
}

/// Port map for the **LEGO Technic Medium Hub** (Control+, set #88012).
///
/// | Port name       | ID |
/// |-----------------|----|
/// | A               |  0 |
/// | B               |  1 |
/// | C               |  2 |
/// | D               |  3 |
/// | HUB_LED         | 50 |
/// | CURRENT_SENSOR  | 59 |
/// | VOLTAGE_SENSOR  | 60 |
/// | ACCELEROMETER   | 97 |
/// | GYRO_SENSOR     | 98 |
/// | TILT_SENSOR     | 99 |
#[must_use]
pub fn technic_medium_hub() -> HashMap<String, u8> {
    HashMap::from([
        ("A".to_owned(), 0),
        ("B".to_owned(), 1),
        ("C".to_owned(), 2),
        ("D".to_owned(), 3),
        ("HUB_LED".to_owned(), 50),
        ("CURRENT_SENSOR".to_owned(), 59),
        ("VOLTAGE_SENSOR".to_owned(), 60),
        ("ACCELEROMETER".to_owned(), 97),
        ("GYRO_SENSOR".to_owned(), 98),
        ("TILT_SENSOR".to_owned(), 99),
    ])
}

/// Port map for the **LEGO Technic Small Hub** (Spike Essential, set #45345).
///
/// | Port name       | ID |
/// |-----------------|----|
/// | A               |  0 |
/// | B               |  1 |
/// | HUB_LED         | 49 |
/// | CURRENT_SENSOR  | 59 |
/// | VOLTAGE_SENSOR  | 60 |
/// | ACCELEROMETER   | 97 |
/// | GYRO_SENSOR     | 98 |
/// | TILT_SENSOR     | 99 |
#[must_use]
pub fn technic_small_hub() -> HashMap<String, u8> {
    HashMap::from([
        ("A".to_owned(), 0),
        ("B".to_owned(), 1),
        ("HUB_LED".to_owned(), 49),
        ("CURRENT_SENSOR".to_owned(), 59),
        ("VOLTAGE_SENSOR".to_owned(), 60),
        ("ACCELEROMETER".to_owned(), 97),
        ("GYRO_SENSOR".to_owned(), 98),
        ("TILT_SENSOR".to_owned(), 99),
    ])
}

/// Port map for the **LEGO Powered UP Remote Control** (#88010).
///
/// | Port name              | ID |
/// |------------------------|----|
/// | LEFT                   |  0 |
/// | RIGHT                  |  1 |
/// | HUB_LED                | 52 |
/// | VOLTAGE_SENSOR         | 59 |
/// | REMOTE_CONTROL_RSSI    | 60 |
#[must_use]
pub fn remote_control() -> HashMap<String, u8> {
    HashMap::from([
        ("LEFT".to_owned(), 0),
        ("RIGHT".to_owned(), 1),
        ("HUB_LED".to_owned(), 52),
        ("VOLTAGE_SENSOR".to_owned(), 59),
        ("REMOTE_CONTROL_RSSI".to_owned(), 60),
    ])
}

/// Port map for the **LEGO DUPLO Train Base** (#10874).
///
/// | Port name       | ID |
/// |-----------------|----|
/// | MOTOR           |  0 |
/// | COLOR           | 18 |
/// | SPEEDOMETER     | 19 |
#[must_use]
pub fn duplo_train_base() -> HashMap<String, u8> {
    HashMap::from([
        ("MOTOR".to_owned(), 0),
        ("COLOR".to_owned(), 18),
        ("SPEEDOMETER".to_owned(), 19),
    ])
}

/// Port map for the **LEGO `WeDo 2.0` Smart Hub** (#45300).
///
/// | Port name       | ID |
/// |-----------------|----|
/// | A               |  1 |
/// | B               |  2 |
/// | CURRENT_SENSOR  |  3 |
/// | VOLTAGE_SENSOR  |  4 |
/// | PIEZO_BUZZER    |  5 |
/// | HUB_LED         |  6 |
#[must_use]
pub fn wedo2_smart_hub() -> HashMap<String, u8> {
    HashMap::from([
        ("A".to_owned(), 1),
        ("B".to_owned(), 2),
        ("CURRENT_SENSOR".to_owned(), 3),
        ("VOLTAGE_SENSOR".to_owned(), 4),
        ("PIEZO_BUZZER".to_owned(), 5),
        ("HUB_LED".to_owned(), 6),
    ])
}
