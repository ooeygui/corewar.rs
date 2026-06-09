//! # CoreWar Protocol
//!
//! Shared WebSocket message definitions used by the server and visualization client.

pub mod encoding;
pub mod error;
pub mod messages;

pub use encoding::*;
pub use error::*;
pub use messages::*;
