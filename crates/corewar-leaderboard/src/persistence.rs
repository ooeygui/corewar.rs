//! File persistence for leaderboard data.

use std::path::Path;

use crate::rating::Leaderboard;

/// Save leaderboard state to a JSON file.
pub fn save_to_file(leaderboard: &Leaderboard, path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(leaderboard.all_ratings())?;
    std::fs::write(path, json)
}

/// Load leaderboard state from a JSON file.
pub fn load_from_file(_path: &Path) -> std::io::Result<Leaderboard> {
    // TODO: Deserialize and rebuild leaderboard
    Ok(Leaderboard::new())
}
