use crate::error::Result;
use bytes::Bytes;
use tokio::sync::mpsc;
use uuid::Uuid;

pub mod mock;

#[cfg(all(target_os = "linux", feature = "hardware-tests"))]
pub mod btleplug;

/// Abstraction over the BLE transport.
/// `MockTransport` implements this for tests; `BtleplugTransport` for real hardware.
///
/// We use `async fn` in trait (AFIT, stable since Rust 1.75). All concrete
/// implementations are `Send`, so their futures are too. The lint is suppressed
/// deliberately.
#[allow(async_fn_in_trait)]
pub trait BleTransport: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn write(&self, characteristic: Uuid, data: Bytes) -> Result<()>;
    /// Returns a channel that delivers raw notification payloads for the given characteristic.
    async fn subscribe(&self, characteristic: Uuid) -> Result<mpsc::Receiver<Bytes>>;
}
