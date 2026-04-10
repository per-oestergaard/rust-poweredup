#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)] // common in Rust crates; not worth fighting

pub mod ble;
pub mod device;
pub mod error;
pub mod hub;
pub mod protocol;
pub mod scanner;
