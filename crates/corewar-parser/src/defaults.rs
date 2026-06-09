use corewar_core::{AddressingMode, Modifier, Opcode};

pub(crate) const fn infer_modifier(
    opcode: Opcode,
    a_mode: AddressingMode,
    b_mode: AddressingMode,
) -> Modifier {
    match opcode {
        Opcode::DAT => Modifier::F,
        Opcode::MOV | Opcode::SEQ | Opcode::SNE => {
            if matches!(a_mode, AddressingMode::Immediate) {
                Modifier::AB
            } else if matches!(b_mode, AddressingMode::Immediate) {
                Modifier::B
            } else {
                Modifier::I
            }
        }
        Opcode::ADD | Opcode::SUB | Opcode::MUL | Opcode::DIV | Opcode::MOD => {
            if matches!(a_mode, AddressingMode::Immediate) {
                Modifier::AB
            } else if matches!(b_mode, AddressingMode::Immediate) {
                Modifier::B
            } else {
                Modifier::F
            }
        }
        Opcode::SLT => {
            if matches!(a_mode, AddressingMode::Immediate) {
                Modifier::AB
            } else {
                Modifier::B
            }
        }
        Opcode::JMP | Opcode::JMZ | Opcode::JMN | Opcode::DJN | Opcode::SPL | Opcode::NOP => {
            Modifier::B
        }
    }
}
