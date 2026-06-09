//! # CoreWar Core
//!
//! Shared types for the CoreWar system: instruction set, addressing modes,
//! modifiers, core memory, and event definitions.
//!
//! This crate is `#[no_std]` compatible (with `alloc`) for WASM builds.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod event;
pub mod instruction;
pub mod memory;
pub mod warrior;

pub use event::{CoreEvent, EventBus};
pub use instruction::{AddressingMode, Instruction, Modifier, Opcode};
pub use memory::Core;
pub use warrior::Warrior;
