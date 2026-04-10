#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("BLE error: {0}")]
    Ble(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
}

pub type Result<T> = std::result::Result<T, Error>;
