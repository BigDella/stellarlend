#![no_std]
#![allow(deprecated)]
pub mod cache;
pub mod events;
pub mod message_bus;
pub mod shared_types;
pub mod storage;
pub mod upgrade;

#[cfg(test)]
mod protocol_integration_test;
