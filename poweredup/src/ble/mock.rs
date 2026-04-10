//! In-process BLE simulator used for all unit and integration tests.
//!
//! `MockTransport` lets tests:
//! - **Pre-script inbound frames** — a queue of `Bytes` the hub would normally send over BLE.
//! - **Capture outbound writes** — every `write()` call is appended to a `Vec` the test can assert.
//!
//! No Bluetooth hardware or OS BLE stack is required.

use std::sync::{Arc, Mutex};

use bytes::Bytes;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::error::{Error, Result};

use super::BleTransport;

// ── MockTransport ─────────────────────────────────────────────────────────────

/// An in-process BLE transport that replaces real hardware in tests.
#[derive(Clone)]
pub struct MockTransport {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    connected: bool,
    /// Messages the mock will deliver to subscribers (one per characteristic UUID).
    inbound: Vec<(Uuid, Bytes)>,
    /// All (characteristic, payload) pairs passed to `write()`.
    pub written: Vec<(Uuid, Bytes)>,
    /// Active subscribers: (uuid, sender).
    senders: Vec<(Uuid, mpsc::Sender<Bytes>)>,
}

impl MockTransport {
    /// Create an unconnected mock transport with no scripted messages.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                connected: false,
                inbound: Vec::new(),
                written: Vec::new(),
                senders: Vec::new(),
            })),
        }
    }

    /// Queue a frame to be delivered to subscribers of `characteristic`.
    /// Call this before connecting, or between operations, to script hub behaviour.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned (only possible if another thread
    /// panicked while holding the lock — does not happen in normal test usage).
    pub fn push_inbound(&self, characteristic: Uuid, data: Bytes) {
        self.inner
            .lock()
            .expect("mock lock poisoned")
            .inbound
            .push((characteristic, data));
    }

    /// Flush all queued inbound frames to their subscribers.
    ///
    /// Tests call this after `connect()` / `subscribe()` to replay scripted messages.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned.
    pub async fn flush_inbound(&self) {
        let frames: Vec<(Uuid, Bytes)> = {
            let mut inner = self.inner.lock().expect("mock lock poisoned");
            std::mem::take(&mut inner.inbound)
        };
        for (uuid, data) in frames {
            let senders: Vec<mpsc::Sender<Bytes>> = self
                .inner
                .lock()
                .expect("mock lock poisoned")
                .senders
                .iter()
                .filter(|(u, _)| *u == uuid)
                .map(|(_, s)| s.clone())
                .collect();
            for sender in senders {
                // Ignore send errors — receiver may have dropped.
                let _ = sender.send(data.clone()).await;
            }
        }
    }

    /// Return a snapshot of all bytes written via `write()`.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned.
    #[must_use]
    pub fn written(&self) -> Vec<(Uuid, Bytes)> {
        self.inner
            .lock()
            .expect("mock lock poisoned")
            .written
            .clone()
    }

    /// Return just the payload bytes written to `characteristic`, in order.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned.
    #[must_use]
    pub fn written_to(&self, characteristic: &Uuid) -> Vec<Bytes> {
        self.inner
            .lock()
            .expect("mock lock poisoned")
            .written
            .iter()
            .filter(|(u, _)| u == characteristic)
            .map(|(_, b)| b.clone())
            .collect()
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl BleTransport for MockTransport {
    async fn connect(&mut self) -> Result<()> {
        let mut inner = self.inner.lock().expect("mock lock poisoned");
        if inner.connected {
            return Err(Error::Ble("already connected".into()));
        }
        inner.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        let mut inner = self.inner.lock().expect("mock lock poisoned");
        inner.connected = false;
        inner.senders.clear();
        Ok(())
    }

    async fn write(&self, characteristic: Uuid, data: Bytes) -> Result<()> {
        let mut inner = self.inner.lock().expect("mock lock poisoned");
        if !inner.connected {
            return Err(Error::Ble("not connected".into()));
        }
        inner.written.push((characteristic, data));
        Ok(())
    }

    async fn subscribe(&self, characteristic: Uuid) -> Result<mpsc::Receiver<Bytes>> {
        let (tx, rx) = mpsc::channel(64);
        let mut inner = self.inner.lock().expect("mock lock poisoned");
        if !inner.connected {
            return Err(Error::Ble("not connected".into()));
        }
        inner.senders.push((characteristic, tx));
        Ok(rx)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn lpf2_all() -> Uuid {
        Uuid::parse_str(crate::protocol::consts::ble_uuid::LPF2_ALL).expect("valid UUID constant")
    }

    #[tokio::test]
    async fn connect_disconnect() {
        let mut t = MockTransport::new();
        t.connect().await.expect("connect");
        t.disconnect().await.expect("disconnect");
    }

    #[tokio::test]
    async fn double_connect_is_error() {
        let mut t = MockTransport::new();
        t.connect().await.expect("first connect");
        assert!(t.connect().await.is_err());
    }

    #[tokio::test]
    async fn write_before_connect_is_error() {
        let t = MockTransport::new();
        let result = t
            .write(lpf2_all(), Bytes::from_static(&[0x05, 0x00, 0x01]))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn captures_written_bytes() {
        let mut t = MockTransport::new();
        t.connect().await.unwrap();
        let uuid = lpf2_all();
        let frame = Bytes::from_static(&[0x05, 0x00, 0x01, 0x02, 0x03]);
        t.write(uuid, frame.clone()).await.unwrap();

        let written = t.written_to(&uuid);
        assert_eq!(written.len(), 1);
        assert_eq!(written[0], frame);
    }

    #[tokio::test]
    async fn delivers_scripted_inbound_messages() {
        let mut t = MockTransport::new();
        let uuid = lpf2_all();
        let frame = Bytes::from_static(&[0x09, 0x00, 0x04, 0x00, 0x01, 0x27, 0x00, 0x00, 0x00]);

        t.push_inbound(uuid, frame.clone());
        t.connect().await.unwrap();
        let mut rx = t.subscribe(uuid).await.unwrap();

        t.flush_inbound().await;

        let received = rx.recv().await.expect("should receive frame");
        assert_eq!(received, frame);
    }
}
