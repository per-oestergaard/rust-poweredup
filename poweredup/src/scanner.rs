//! BLE advertisement parsing and hub-type detection.
//!
//! The scanner layer sits between the raw BLE transport and the hub typestate.
//! It is responsible for one thing: given a BLE advertisement, determine which
//! hub model it came from and hand back a ready-to-connect
//! [`Hub<Disconnected, T>`](crate::hub::Hub) of the correct type.
//!
//! # Workflow
//!
//! 1. The host BLE stack delivers advertisement events (service UUIDs + manufacturer data).
//! 2. Call [`AdvertisedHub::from_advertisement`] to parse the advertisement.
//! 3. If `Some(hub)` is returned, call [`AdvertisedHub::into_hub`] with a transport
//!    to get a fully-configured `Hub<Disconnected, T>`.
//!
//! # Hardware scanning
//!
//! Actual BLE scanning (starting / stopping the adapter, receiving peripherals) is
//! done by the caller — either the `btleplug` backend (feature `hardware-tests`) or a
//! test harness.  This module is intentionally transport-agnostic.

use crate::{
    ble::BleTransport,
    hub::{Disconnected, Hub},
    protocol::consts::HubType,
};

/// The result of successfully parsing a BLE advertisement.
///
/// Wraps the detected [`HubType`] so the caller can inspect it before
/// committing to a transport connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdvertisedHub {
    /// Hub model detected from the advertisement.
    pub hub_type: HubType,
}

impl AdvertisedHub {
    /// Parse a BLE advertisement and return an `AdvertisedHub` if it matches a
    /// known LEGO hub.
    ///
    /// * `service_uuids` — 128-bit service UUIDs (with or without dashes) from
    ///   the advertisement.
    /// * `manufacturer_data` — raw manufacturer-specific advertisement bytes.
    ///   Byte at index 3 carries the LPF2 hub model discriminant.
    ///
    /// Returns `None` when the advertisement does not match any known hub.
    #[must_use]
    pub fn from_advertisement(service_uuids: &[&str], manufacturer_data: &[u8]) -> Option<Self> {
        HubType::from_advertisement(service_uuids, manufacturer_data)
            .map(|hub_type| Self { hub_type })
    }

    /// Construct a [`Hub<Disconnected, T>`] pre-configured for this hub model.
    ///
    /// The returned hub is ready to call `.connect().await` on.
    #[must_use]
    pub fn into_hub<T: BleTransport>(self, transport: T) -> Hub<Disconnected, T> {
        match self.hub_type {
            HubType::Hub => Hub::powered_up_hub(transport),
            HubType::MoveHub => Hub::move_hub(transport),
            HubType::TechnicMediumHub => Hub::technic_medium_hub(transport),
            HubType::TechnicSmallHub => Hub::technic_small_hub(transport),
            HubType::RemoteControl => Hub::remote_control(transport),
            HubType::DuploTrainBase => Hub::duplo_train_base(transport),
            HubType::WeDo2SmartHub => Hub::wedo2_smart_hub(transport),
            // Mario/Luigi/Peach use the generic LPF2 hub port map for now.
            HubType::Mario | HubType::Luigi | HubType::Peach | HubType::Unknown => {
                Hub::new(transport, self.hub_type, std::collections::HashMap::new())
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ble::mock::MockTransport;
    use crate::protocol::consts::ble_uuid;

    fn lpf2_services() -> Vec<&'static str> {
        vec![ble_uuid::LPF2_SERVICE]
    }

    fn wedo2_services() -> Vec<&'static str> {
        vec![ble_uuid::WEDO2_SERVICE]
    }

    fn mfr(id_byte: u8) -> Vec<u8> {
        // Manufacturer data: bytes 0-2 can be anything; byte 3 is the hub ID.
        vec![0x97, 0x03, 0x00, id_byte]
    }

    #[test]
    fn parse_move_hub() {
        let adv = AdvertisedHub::from_advertisement(&lpf2_services(), &mfr(64)).unwrap();
        assert_eq!(adv.hub_type, HubType::MoveHub);
    }

    #[test]
    fn parse_powered_up_hub() {
        let adv = AdvertisedHub::from_advertisement(&lpf2_services(), &mfr(65)).unwrap();
        assert_eq!(adv.hub_type, HubType::Hub);
    }

    #[test]
    fn parse_remote_control() {
        let adv = AdvertisedHub::from_advertisement(&lpf2_services(), &mfr(66)).unwrap();
        assert_eq!(adv.hub_type, HubType::RemoteControl);
    }

    #[test]
    fn parse_technic_medium_hub() {
        let adv = AdvertisedHub::from_advertisement(&lpf2_services(), &mfr(128)).unwrap();
        assert_eq!(adv.hub_type, HubType::TechnicMediumHub);
    }

    #[test]
    fn parse_technic_small_hub() {
        let adv = AdvertisedHub::from_advertisement(&lpf2_services(), &mfr(131)).unwrap();
        assert_eq!(adv.hub_type, HubType::TechnicSmallHub);
    }

    #[test]
    fn parse_duplo_train_base() {
        let adv = AdvertisedHub::from_advertisement(&lpf2_services(), &mfr(32)).unwrap();
        assert_eq!(adv.hub_type, HubType::DuploTrainBase);
    }

    #[test]
    fn parse_wedo2_smart_hub() {
        // WeDo 2.0 is identified by service UUID alone — no manufacturer byte needed.
        let adv = AdvertisedHub::from_advertisement(&wedo2_services(), &[]).unwrap();
        assert_eq!(adv.hub_type, HubType::WeDo2SmartHub);
    }

    #[test]
    fn unknown_service_returns_none() {
        let adv =
            AdvertisedHub::from_advertisement(&["00001800-0000-1000-8000-00805f9b34fb"], &mfr(65));
        assert!(adv.is_none());
    }

    #[test]
    fn unknown_manufacturer_byte_returns_none() {
        // LPF2 service present but an unrecognised ID byte.
        let adv = AdvertisedHub::from_advertisement(&lpf2_services(), &mfr(0xFF));
        assert!(adv.is_none());
    }

    #[test]
    fn into_hub_powered_up_hub() {
        let adv = AdvertisedHub::from_advertisement(&lpf2_services(), &mfr(65)).unwrap();
        let hub = adv.into_hub(MockTransport::new());
        assert_eq!(hub.hub_type(), HubType::Hub);
        // Verify the port map is pre-populated.
        assert_eq!(hub.port_id("A"), Some(0u8));
        assert_eq!(hub.port_id("HUB_LED"), Some(50u8));
    }

    #[test]
    fn into_hub_move_hub() {
        let adv = AdvertisedHub::from_advertisement(&lpf2_services(), &mfr(64)).unwrap();
        let hub = adv.into_hub(MockTransport::new());
        assert_eq!(hub.hub_type(), HubType::MoveHub);
        assert_eq!(hub.port_id("TILT_SENSOR"), Some(58u8));
    }

    #[test]
    fn into_hub_remote_control() {
        let adv = AdvertisedHub::from_advertisement(&lpf2_services(), &mfr(66)).unwrap();
        let hub = adv.into_hub(MockTransport::new());
        assert_eq!(hub.hub_type(), HubType::RemoteControl);
        assert_eq!(hub.port_id("LEFT"), Some(0u8));
        assert_eq!(hub.port_id("RIGHT"), Some(1u8));
    }

    #[test]
    fn into_hub_wedo2() {
        let adv = AdvertisedHub::from_advertisement(&wedo2_services(), &[]).unwrap();
        let hub = adv.into_hub(MockTransport::new());
        assert_eq!(hub.hub_type(), HubType::WeDo2SmartHub);
        assert_eq!(hub.port_id("A"), Some(1u8));
    }
}
