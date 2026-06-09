//! Default ICWS'94 runtime constants.

/// Default core size.
pub const CORESIZE: usize = 8_000;
/// Default maximum cycle count before a tie is declared.
pub const MAXCYCLES: usize = 80_000;
/// Default maximum processes per warrior.
pub const MAXPROCESSES: usize = 8_000;
/// Default maximum warrior length in instructions.
pub const MAXLENGTH: usize = 100;
/// Default minimum load distance between warriors.
pub const MINDISTANCE: usize = 100;
/// Default warrior count for a standard match.
pub const MAXWARRIORS: usize = 2;
