//! # CoreWar Leaderboard
//!
//! In-memory rating system with optional file persistence.
//! Implements Glicko-2 rating for warrior rankings.

pub mod persistence;
pub mod rating;

pub use rating::{Leaderboard, PlayerRating};
