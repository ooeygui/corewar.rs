//! Main parser entry point for Redcode warrior files.

use corewar_core::Warrior;

use crate::error::ParseError;

/// Parse a Redcode source string into a Warrior.
///
/// Supports ICWS'94 standard with pMARS extensions including:
/// - Labels and EQU definitions
/// - All addressing modes (#, $, @, <, >, {, })
/// - All modifiers (.A, .B, .AB, .BA, .F, .X, .I)
/// - Predefined constants (CORESIZE, MAXPROCESSES, etc.)
/// - Metadata directives (;name, ;author, ;strategy)
/// - ORG and END directives
pub fn parse_warrior(_source: &str) -> Result<Warrior, ParseError> {
    // TODO: Implement full parser
    Ok(Warrior::new("unnamed", Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = parse_warrior("");
        assert!(result.is_ok());
    }
}
