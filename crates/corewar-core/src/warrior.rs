#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use serde::{Deserialize, Serialize};

use crate::instruction::Instruction;

/// Metadata and loaded instructions for a warrior program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warrior {
    /// Warrior name (from ;name directive).
    pub name: String,
    /// Author (from ;author directive).
    pub author: String,
    /// Strategy description (from ;strategy directive).
    pub strategy: Option<String>,
    /// Assembled instructions.
    pub instructions: Vec<Instruction>,
    /// Entry point offset (ORG/START).
    pub start_offset: usize,
    /// Unique identifier assigned during battle.
    #[serde(skip)]
    pub id: u32,
}

impl Warrior {
    pub fn new(name: impl Into<String>, instructions: Vec<Instruction>) -> Self {
        Self {
            name: name.into(),
            author: String::new(),
            strategy: None,
            instructions,
            start_offset: 0,
            id: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}
