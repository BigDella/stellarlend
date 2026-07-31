#![no_std]

pub mod error;
pub mod events;

pub use error::{
    CrossContractError, ErrorKind, InitializationError, ProtocolError, StorageError,
    ValidationError,
};
