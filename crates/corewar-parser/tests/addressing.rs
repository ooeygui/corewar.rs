use corewar_core::{AddressingMode, Instruction, Modifier, Opcode};
use corewar_parser::parse_warrior;

#[test]
fn parses_all_addressing_modes() {
    let warrior = parse_warrior("MOV #1, $2\nMOV @3, <4\nMOV >5, {6\nMOV }7, *8\n").unwrap();

    assert_eq!(warrior.instructions.len(), 4);
    assert_eq!(
        warrior.instructions[0],
        Instruction::new(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            1,
            AddressingMode::Direct,
            2,
        )
    );
    assert_eq!(
        warrior.instructions[1],
        Instruction::new(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::IndirectB,
            3,
            AddressingMode::PreDecIndirectB,
            4,
        )
    );
    assert_eq!(
        warrior.instructions[2],
        Instruction::new(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::PostIncIndirectB,
            5,
            AddressingMode::PreDecIndirectA,
            6,
        )
    );
    assert_eq!(
        warrior.instructions[3],
        Instruction::new(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::PostIncIndirectA,
            7,
            AddressingMode::IndirectA,
            8,
        )
    );
}

#[test]
fn defaults_to_direct_addressing_without_prefix() {
    let warrior = parse_warrior("MOV 5, 6\n").unwrap();

    assert_eq!(warrior.instructions[0].a_mode, AddressingMode::Direct);
    assert_eq!(warrior.instructions[0].a_value, 5);
    assert_eq!(warrior.instructions[0].b_mode, AddressingMode::Direct);
    assert_eq!(warrior.instructions[0].b_value, 6);
}

#[test]
fn parses_negative_operand_values() {
    let warrior = parse_warrior("ADD #-7, $-11\n").unwrap();

    assert_eq!(
        warrior.instructions[0],
        Instruction::new(
            Opcode::ADD,
            Modifier::AB,
            AddressingMode::Immediate,
            -7,
            AddressingMode::Direct,
            -11,
        )
    );
}

#[test]
fn parses_large_operand_values() {
    let warrior = parse_warrior("MOV #123456, $654321\n").unwrap();

    assert_eq!(
        warrior.instructions[0],
        Instruction::new(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            123456,
            AddressingMode::Direct,
            654321,
        )
    );
}
