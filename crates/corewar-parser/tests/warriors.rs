use std::{fs, path::PathBuf};

use corewar_core::{AddressingMode, Instruction, Modifier, Opcode};
use corewar_parser::parse_warrior;

fn parse_file(name: &str) -> corewar_core::Warrior {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../warriors")
        .join(name);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    parse_warrior(&source).unwrap()
}

fn assert_instruction(
    instruction: &Instruction,
    opcode: Opcode,
    modifier: Modifier,
    a_mode: AddressingMode,
    a_value: i32,
    b_mode: AddressingMode,
    b_value: i32,
) {
    assert_eq!(
        *instruction,
        Instruction::new(opcode, modifier, a_mode, a_value, b_mode, b_value)
    );
}

#[test]
fn parses_imp_source() {
    let warrior = parse_warrior("MOV 0, 1\n").unwrap();

    assert_eq!(warrior.instructions.len(), 1);
    assert_instruction(
        &warrior.instructions[0],
        Opcode::MOV,
        Modifier::I,
        AddressingMode::Direct,
        0,
        AddressingMode::Direct,
        1,
    );
}

#[test]
fn parses_imp_file() {
    let warrior = parse_file("imp.red");

    assert_eq!(warrior.name, "Imp");
    assert_eq!(warrior.author, "A.K. Dewdney");
    assert_eq!(warrior.instructions.len(), 1);
    assert_instruction(
        &warrior.instructions[0],
        Opcode::MOV,
        Modifier::I,
        AddressingMode::Direct,
        0,
        AddressingMode::Direct,
        1,
    );
}

#[test]
fn parses_dwarf_file() {
    let warrior = parse_file("dwarf.red");

    assert_eq!(warrior.name, "Dwarf");
    assert_eq!(warrior.author, "A.K. Dewdney");
    assert_eq!(warrior.instructions.len(), 4);
    assert_instruction(
        &warrior.instructions[0],
        Opcode::ADD,
        Modifier::AB,
        AddressingMode::Immediate,
        4,
        AddressingMode::Direct,
        3,
    );
    assert_instruction(
        &warrior.instructions[1],
        Opcode::MOV,
        Modifier::I,
        AddressingMode::Direct,
        2,
        AddressingMode::IndirectB,
        2,
    );
    assert_instruction(
        &warrior.instructions[2],
        Opcode::JMP,
        Modifier::B,
        AddressingMode::Direct,
        -2,
        AddressingMode::Direct,
        0,
    );
    assert_instruction(
        &warrior.instructions[3],
        Opcode::DAT,
        Modifier::F,
        AddressingMode::Immediate,
        0,
        AddressingMode::Immediate,
        0,
    );
}

#[test]
fn parses_mice_file() {
    let warrior = parse_file("mice.red");

    assert_eq!(warrior.name, "Mice");
    assert_eq!(warrior.author, "Chip Wendell");
    assert_eq!(warrior.instructions.len(), 7);
    assert_instruction(
        &warrior.instructions[0],
        Opcode::MOV,
        Modifier::AB,
        AddressingMode::Immediate,
        12,
        AddressingMode::Direct,
        -1,
    );
    assert_instruction(
        &warrior.instructions[1],
        Opcode::MOV,
        Modifier::I,
        AddressingMode::IndirectB,
        -2,
        AddressingMode::PreDecIndirectB,
        5,
    );
    assert_instruction(
        &warrior.instructions[2],
        Opcode::DJN,
        Modifier::F,
        AddressingMode::Direct,
        -1,
        AddressingMode::Direct,
        -3,
    );
    assert_instruction(
        &warrior.instructions[3],
        Opcode::SPL,
        Modifier::B,
        AddressingMode::IndirectB,
        3,
        AddressingMode::Direct,
        0,
    );
    assert_instruction(
        &warrior.instructions[4],
        Opcode::ADD,
        Modifier::AB,
        AddressingMode::Immediate,
        653,
        AddressingMode::Direct,
        2,
    );
    assert_instruction(
        &warrior.instructions[5],
        Opcode::JMZ,
        Modifier::F,
        AddressingMode::Direct,
        -5,
        AddressingMode::Direct,
        -6,
    );
    assert_instruction(
        &warrior.instructions[6],
        Opcode::DAT,
        Modifier::F,
        AddressingMode::Immediate,
        0,
        AddressingMode::Immediate,
        833,
    );
}
