use serde::{Deserialize, Serialize};

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

/// Addressing modes as defined in ICWS'94 + pMARS extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AddressingMode {
    /// `#` — Immediate
    Immediate,
    /// `$` — Direct
    Direct,
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

/// A single CoreWar instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instruction {
    pub opcode: Opcode,
    pub modifier: Modifier,
    pub a_mode: AddressingMode,
    pub a_value: i32,
    pub b_mode: AddressingMode,
    pub b_value: i32,
}

impl Instruction {
    pub const fn default_dat() -> Self {
        Self {
            opcode: Opcode::DAT,
            modifier: Modifier::F,
            a_mode: AddressingMode::Direct,
            a_value: 0,
            b_mode: AddressingMode::Direct,
            b_value: 0,
        }
    }
}

impl Default for Instruction {
    fn default() -> Self {
        Self::default_dat()
    }
}
