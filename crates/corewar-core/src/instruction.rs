use core::fmt;
use core::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseInstructionComponentError {
    component: &'static str,
}

impl ParseInstructionComponentError {
    const fn new(component: &'static str) -> Self {
        Self { component }
    }
}

impl fmt::Display for ParseInstructionComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid {}", self.component)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseInstructionComponentError {}

/// The 16 ICWS'94 opcodes plus NOP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Opcode {
    DAT,
    MOV,
    ADD,
    SUB,
    MUL,
    DIV,
    MOD,
    JMP,
    JMZ,
    JMN,
    DJN,
    SEQ,
    SNE,
    SLT,
    SPL,
    NOP,
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DAT => "DAT",
            Self::MOV => "MOV",
            Self::ADD => "ADD",
            Self::SUB => "SUB",
            Self::MUL => "MUL",
            Self::DIV => "DIV",
            Self::MOD => "MOD",
            Self::JMP => "JMP",
            Self::JMZ => "JMZ",
            Self::JMN => "JMN",
            Self::DJN => "DJN",
            Self::SEQ => "SEQ",
            Self::SNE => "SNE",
            Self::SLT => "SLT",
            Self::SPL => "SPL",
            Self::NOP => "NOP",
        })
    }
}

impl FromStr for Opcode {
    type Err = ParseInstructionComponentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.eq_ignore_ascii_case("DAT") {
            Ok(Self::DAT)
        } else if s.eq_ignore_ascii_case("MOV") {
            Ok(Self::MOV)
        } else if s.eq_ignore_ascii_case("ADD") {
            Ok(Self::ADD)
        } else if s.eq_ignore_ascii_case("SUB") {
            Ok(Self::SUB)
        } else if s.eq_ignore_ascii_case("MUL") {
            Ok(Self::MUL)
        } else if s.eq_ignore_ascii_case("DIV") {
            Ok(Self::DIV)
        } else if s.eq_ignore_ascii_case("MOD") {
            Ok(Self::MOD)
        } else if s.eq_ignore_ascii_case("JMP") {
            Ok(Self::JMP)
        } else if s.eq_ignore_ascii_case("JMZ") {
            Ok(Self::JMZ)
        } else if s.eq_ignore_ascii_case("JMN") {
            Ok(Self::JMN)
        } else if s.eq_ignore_ascii_case("DJN") {
            Ok(Self::DJN)
        } else if s.eq_ignore_ascii_case("SEQ") || s.eq_ignore_ascii_case("CMP") {
            Ok(Self::SEQ)
        } else if s.eq_ignore_ascii_case("SNE") {
            Ok(Self::SNE)
        } else if s.eq_ignore_ascii_case("SLT") {
            Ok(Self::SLT)
        } else if s.eq_ignore_ascii_case("SPL") {
            Ok(Self::SPL)
        } else if s.eq_ignore_ascii_case("NOP") {
            Ok(Self::NOP)
        } else {
            Err(ParseInstructionComponentError::new("opcode"))
        }
    }
}

/// Addressing modes as defined in ICWS'94 + pMARS extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AddressingMode {
    /// `#` — Immediate
    Immediate,
    /// `$` — Direct
    Direct,
    /// `*` — A-field indirect
    IndirectA,
    /// `@` — B-field indirect
    IndirectB,
    /// `<` — B-field indirect with pre-decrement
    PreDecIndirectB,
    /// `>` — B-field indirect with post-increment
    PostIncIndirectB,
    /// `{` — A-field indirect with pre-decrement
    PreDecIndirectA,
    /// `}` — A-field indirect with post-increment
    PostIncIndirectA,
}

impl Default for AddressingMode {
    fn default() -> Self {
        Self::Direct
    }
}

impl fmt::Display for AddressingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Immediate => "#",
            Self::Direct => "$",
            Self::IndirectA => "*",
            Self::IndirectB => "@",
            Self::PreDecIndirectB => "<",
            Self::PostIncIndirectB => ">",
            Self::PreDecIndirectA => "{",
            Self::PostIncIndirectA => "}",
        })
    }
}

impl FromStr for AddressingMode {
    type Err = ParseInstructionComponentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        match s {
            "#" => Ok(Self::Immediate),
            "$" => Ok(Self::Direct),
            "*" => Ok(Self::IndirectA),
            "@" => Ok(Self::IndirectB),
            "<" => Ok(Self::PreDecIndirectB),
            ">" => Ok(Self::PostIncIndirectB),
            "{" => Ok(Self::PreDecIndirectA),
            "}" => Ok(Self::PostIncIndirectA),
            _ if s.eq_ignore_ascii_case("IMMEDIATE") => Ok(Self::Immediate),
            _ if s.eq_ignore_ascii_case("DIRECT") => Ok(Self::Direct),
            _ if s.eq_ignore_ascii_case("INDIRECTA") || s.eq_ignore_ascii_case("AINDIRECT") => {
                Ok(Self::IndirectA)
            }
            _ if s.eq_ignore_ascii_case("INDIRECTB") || s.eq_ignore_ascii_case("BINDIRECT") => {
                Ok(Self::IndirectB)
            }
            _ if s.eq_ignore_ascii_case("PREDECINDIRECTB")
                || s.eq_ignore_ascii_case("BPREDECINDIRECT") =>
            {
                Ok(Self::PreDecIndirectB)
            }
            _ if s.eq_ignore_ascii_case("POSTINCINDIRECTB")
                || s.eq_ignore_ascii_case("BPOSTINCINDIRECT") =>
            {
                Ok(Self::PostIncIndirectB)
            }
            _ if s.eq_ignore_ascii_case("PREDECINDIRECTA")
                || s.eq_ignore_ascii_case("APREDECINDIRECT") =>
            {
                Ok(Self::PreDecIndirectA)
            }
            _ if s.eq_ignore_ascii_case("POSTINCINDIRECTA")
                || s.eq_ignore_ascii_case("APOSTINCINDIRECT") =>
            {
                Ok(Self::PostIncIndirectA)
            }
            _ => Err(ParseInstructionComponentError::new("addressing mode")),
        }
    }
}

/// Instruction modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Modifier {
    A,
    B,
    AB,
    BA,
    F,
    X,
    I,
}

impl Default for Modifier {
    fn default() -> Self {
        Self::F
    }
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::A => "A",
            Self::B => "B",
            Self::AB => "AB",
            Self::BA => "BA",
            Self::F => "F",
            Self::X => "X",
            Self::I => "I",
        })
    }
}

impl FromStr for Modifier {
    type Err = ParseInstructionComponentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let s = s.strip_prefix('.').unwrap_or(s);

        if s.eq_ignore_ascii_case("A") {
            Ok(Self::A)
        } else if s.eq_ignore_ascii_case("B") {
            Ok(Self::B)
        } else if s.eq_ignore_ascii_case("AB") {
            Ok(Self::AB)
        } else if s.eq_ignore_ascii_case("BA") {
            Ok(Self::BA)
        } else if s.eq_ignore_ascii_case("F") {
            Ok(Self::F)
        } else if s.eq_ignore_ascii_case("X") {
            Ok(Self::X)
        } else if s.eq_ignore_ascii_case("I") {
            Ok(Self::I)
        } else {
            Err(ParseInstructionComponentError::new("modifier"))
        }
    }
}

/// A single CoreWar instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Instruction {
    pub opcode: Opcode,
    pub modifier: Modifier,
    pub a_mode: AddressingMode,
    pub a_value: i32,
    pub b_mode: AddressingMode,
    pub b_value: i32,
}

impl Instruction {
    pub const fn new(
        opcode: Opcode,
        modifier: Modifier,
        a_mode: AddressingMode,
        a_value: i32,
        b_mode: AddressingMode,
        b_value: i32,
    ) -> Self {
        Self {
            opcode,
            modifier,
            a_mode,
            a_value,
            b_mode,
            b_value,
        }
    }

    pub const fn default_dat() -> Self {
        Self::new(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            0,
        )
    }
}

impl Default for Instruction {
    fn default() -> Self {
        Self::default_dat()
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{} {}{}, {}{}",
            self.opcode, self.modifier, self.a_mode, self.a_value, self.b_mode, self.b_value
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_parses_case_insensitively() {
        assert_eq!(Opcode::from_str("mov").unwrap(), Opcode::MOV);
        assert_eq!(Opcode::from_str("cmp").unwrap(), Opcode::SEQ);
    }

    #[test]
    fn addressing_mode_supports_all_symbols() {
        assert_eq!(
            AddressingMode::from_str("*").unwrap(),
            AddressingMode::IndirectA
        );
        assert_eq!(AddressingMode::IndirectA.to_string(), "*");
        assert_eq!(AddressingMode::default(), AddressingMode::Direct);
    }

    #[test]
    fn modifier_accepts_optional_dot() {
        assert_eq!(Modifier::from_str(".i").unwrap(), Modifier::I);
        assert_eq!(Modifier::default(), Modifier::F);
    }

    #[test]
    fn instruction_formats_like_pmars() {
        let instruction = Instruction::new(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            1,
        );

        assert_eq!(instruction.to_string(), "MOV.I $0, $1");
    }
}
