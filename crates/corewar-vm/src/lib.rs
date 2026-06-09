//! # CoreWar VM
//!
//! MARS (Memory Array Redcode Simulator) implementation.
//! Executes warriors in a shared core memory with round-robin process scheduling.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod address;
pub mod battle;
pub mod config;
pub mod executor;
pub mod recorder;
pub mod replay;
#[cfg(feature = "wasm")]
pub mod wasm_api;
#[cfg(feature = "wasm")]
pub mod wasm_utils;

pub use battle::{
    Battle, BattleConfig, BattleObserver, BattleResult, BattleSetup, BattleStats, RoundResult,
    ScoringMode, WarriorPlacement,
};
pub use config::VmConfig;
pub use executor::Executor;
pub use recorder::{EventRecorder, EventSnapshot};
pub use replay::{Replay, ReplayBuilder};
