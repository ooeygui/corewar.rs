//! File persistence for leaderboard data.

use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::rating::Leaderboard;

/// Save leaderboard state to a JSON file.
pub fn save_to_file(leaderboard: &Leaderboard, path: &Path) -> io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_vec_pretty(leaderboard).map_err(io::Error::other)?;
    std::fs::write(path, json)
}

/// Load leaderboard state from a JSON file.
pub fn load_from_file(path: &Path) -> io::Result<Leaderboard> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Leaderboard::new()),
        Err(error) => Err(error),
    }
}

/// Save leaderboard state to a MessagePack file.
pub fn save_to_file_msgpack(leaderboard: &Leaderboard, path: &Path) -> io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }

    let payload = rmp_serde::to_vec_named(leaderboard).map_err(io::Error::other)?;
    std::fs::write(path, payload)
}

/// Load leaderboard state from a MessagePack file.
pub fn load_from_file_msgpack(path: &Path) -> io::Result<Leaderboard> {
    match std::fs::read(path) {
        Ok(bytes) => rmp_serde::from_slice(&bytes).map_err(io::Error::other),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Leaderboard::new()),
        Err(error) => Err(error),
    }
}

/// Save a timestamped autosave snapshot and return the path used.
pub fn auto_save(leaderboard: &Leaderboard, directory: &Path) -> io::Result<PathBuf> {
    std::fs::create_dir_all(directory)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = directory.join(format!("leaderboard-autosave-{timestamp}.json"));
    save_to_file(leaderboard, &path)?;
    Ok(path)
}
