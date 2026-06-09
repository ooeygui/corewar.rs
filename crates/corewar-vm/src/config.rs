use serde::{Deserialize, Serialize};

/// Configuration for the MARS virtual machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Size of the core memory array.
    pub core_size: usize,
    /// Maximum number of cycles before a draw is declared.
    pub max_cycles: u64,
    /// Maximum number of processes per warrior.
    pub max_processes: usize,
    /// Maximum warrior length in instructions.
    pub max_length: usize,
    /// Minimum distance between warriors when loaded.
    pub min_distance: usize,
    /// Random seed for warrior placement.
    pub seed: u64,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            core_size: 8000,
            max_cycles: 80_000,
            max_processes: 8000,
            max_length: 100,
            min_distance: 100,
            seed: 42,
        }
    }
}
