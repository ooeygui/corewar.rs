//! # CoreWar VM
//!
//! MARS (Memory Array Redcode Simulator) implementation.
//! Executes warriors in a shared core memory with round-robin process scheduling.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod battle;
pub mod config;
pub mod executor;

pub use battle::Battle;
pub use config::VmConfig;
pub use executor::Executor;
