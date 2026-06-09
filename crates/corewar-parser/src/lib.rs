//! # CoreWar Parser
//!
//! Redcode load file parser implementing ICWS'94 standard with pMARS extensions.
//! Handles label resolution, EQU/constant expansion, and predefined constants.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod defaults;
pub mod error;
pub mod lexer;
pub mod parser;

pub use error::ParseError;
pub use parser::parse_warrior;
