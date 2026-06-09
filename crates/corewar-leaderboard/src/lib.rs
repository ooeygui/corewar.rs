//! # CoreWar Leaderboard
//!
//! In-memory rating system with optional file persistence.
//! Implements Glicko-2 rating for warrior rankings.

pub mod history;
pub mod persistence;
pub mod rating;

pub use history::{HistoricalMatch, MatchHistory};
pub use persistence::{
    auto_save, load_from_file, load_from_file_msgpack, save_to_file, save_to_file_msgpack,
};
pub use rating::{HeadToHead, Leaderboard, MatchOutcome, PlayerRating};

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::persistence::{
        auto_save, load_from_file, load_from_file_msgpack, save_to_file, save_to_file_msgpack,
    };
    use crate::{HeadToHead, Leaderboard};

    fn test_path(extension: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("target")
            .join("corewar-leaderboard-tests")
            .join(format!("leaderboard-{unique}.{extension}"))
    }

    #[test]
    fn recording_a_win_updates_ratings() {
        let mut leaderboard = Leaderboard::new();
        leaderboard.record_result("alpha", "beta");

        let winner = leaderboard.player("alpha").expect("winner should exist");
        let loser = leaderboard.player("beta").expect("loser should exist");

        assert!(winner.rating > 1500.0);
        assert!(loser.rating < 1500.0);
        assert_eq!(winner.games_played(), 1);
        assert_eq!(loser.games_played(), 1);
    }

    #[test]
    fn recording_a_draw_tracks_stats() {
        let mut leaderboard = Leaderboard::new();
        leaderboard.record_draw("alpha", "beta");

        let alpha = leaderboard.player("alpha").expect("alpha should exist");
        let beta = leaderboard.player("beta").expect("beta should exist");

        assert!((alpha.rating - 1500.0).abs() < 0.000_001);
        assert!((beta.rating - 1500.0).abs() < 0.000_001);
        assert_eq!(alpha.draws, 1);
        assert_eq!(beta.draws, 1);
        assert_eq!(alpha.games_played(), 1);
        assert_eq!(beta.games_played(), 1);
    }

    #[test]
    fn loading_missing_files_returns_empty_leaderboard() {
        let json_path = test_path("missing-json");
        let msgpack_path = test_path("missing-msgpack");

        let loaded_json = load_from_file(&json_path).expect("missing json should load empty");
        let loaded_msgpack =
            load_from_file_msgpack(&msgpack_path).expect("missing msgpack should load empty");

        assert!(loaded_json.all_ratings().is_empty());
        assert!(loaded_msgpack.all_ratings().is_empty());
    }

    #[test]
    fn persistence_round_trip_preserves_leaderboard() {
        let json_path = test_path("json");
        let msgpack_path = test_path("msgpack");
        let autosave_dir = json_path
            .parent()
            .expect("json path should have a parent")
            .join("autosave");

        let mut leaderboard = Leaderboard::new();
        leaderboard.record_result("alpha", "beta");
        leaderboard.record_draw("alpha", "beta");
        leaderboard.record_result("gamma", "alpha");

        save_to_file(&leaderboard, &json_path).expect("json save should succeed");
        let loaded_json = load_from_file(&json_path).expect("json load should succeed");
        assert_eq!(loaded_json.all_ratings(), leaderboard.all_ratings());
        assert_eq!(loaded_json.history(), leaderboard.history());
        assert_eq!(
            loaded_json.head_to_head("alpha", "beta"),
            leaderboard.head_to_head("alpha", "beta")
        );

        save_to_file_msgpack(&leaderboard, &msgpack_path).expect("msgpack save should succeed");
        let loaded_msgpack =
            load_from_file_msgpack(&msgpack_path).expect("msgpack load should succeed");
        assert_eq!(loaded_msgpack.all_ratings(), leaderboard.all_ratings());
        assert_eq!(loaded_msgpack.history(), leaderboard.history());

        let autosave_path =
            auto_save(&leaderboard, &autosave_dir).expect("autosave should succeed");
        assert!(autosave_path.exists());

        let _ = std::fs::remove_file(&json_path);
        let _ = std::fs::remove_file(&msgpack_path);
        let _ = std::fs::remove_file(&autosave_path);
        let _ = std::fs::remove_dir_all(&autosave_dir);
    }

    #[test]
    fn head_to_head_stats_are_symmetric() {
        let mut leaderboard = Leaderboard::new();
        leaderboard.record_result("alpha", "beta");
        leaderboard.record_result("alpha", "beta");
        leaderboard.record_result("beta", "alpha");
        leaderboard.record_draw("alpha", "beta");

        assert_eq!(
            leaderboard.head_to_head("alpha", "beta"),
            HeadToHead {
                wins_a: 2,
                wins_b: 1,
                draws: 1,
            }
        );
        assert_eq!(
            leaderboard.head_to_head("beta", "alpha"),
            HeadToHead {
                wins_a: 1,
                wins_b: 2,
                draws: 1,
            }
        );
    }

    #[test]
    fn recent_match_history_filters_by_player() {
        let mut leaderboard = Leaderboard::new();
        leaderboard.record_result("alpha", "beta");
        leaderboard.record_draw("alpha", "gamma");
        leaderboard.record_result("gamma", "beta");

        let recent = leaderboard.history().recent_matches_for_player("alpha", 2);
        assert_eq!(recent.len(), 2);
        assert!(recent
            .iter()
            .all(|entry| entry.outcome.involves_player("alpha")));
    }
}
