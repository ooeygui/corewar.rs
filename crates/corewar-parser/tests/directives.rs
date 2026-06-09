use corewar_core::{AddressingMode, Instruction, Modifier, Opcode};
use corewar_parser::parse_warrior;

#[test]
fn extracts_name_and_author_metadata() {
    let warrior = parse_warrior(";name Test Warrior\n;author Parser Tester\nMOV 0, 1\n").unwrap();

    assert_eq!(warrior.name, "Test Warrior");
    assert_eq!(warrior.author, "Parser Tester");
}

#[test]
fn org_sets_start_offset() {
    let warrior = parse_warrior("MOV 0, 1\nDAT #0, #0\nORG 1\n").unwrap();

    assert_eq!(warrior.start_offset, 1);
}

#[test]
fn end_with_label_sets_start_offset() {
    let warrior = parse_warrior("start MOV 0, 1\ntarget DAT #0, #0\nEND target\n").unwrap();

    assert_eq!(warrior.start_offset, 1);
}

#[test]
fn equ_expands_constants() {
    let warrior = parse_warrior("step EQU (2 + 3) * 4\nMOV #step, $step\n").unwrap();

    assert_eq!(
        warrior.instructions[0],
        Instruction::new(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            20,
            AddressingMode::Direct,
            20,
        )
    );
}

#[test]
fn ignores_full_line_and_inline_comments() {
    let warrior = parse_warrior(
        "; this is a comment\nMOV 0, 1 ; copy forward\n; another comment\nDAT #0, #0 ; bomb\n",
    )
    .unwrap();

    assert_eq!(warrior.instructions.len(), 2);
    assert_eq!(
        warrior.instructions[0],
        Instruction::new(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            1,
        )
    );
    assert_eq!(
        warrior.instructions[1],
        Instruction::new(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Immediate,
            0,
            AddressingMode::Immediate,
            0,
        )
    );
}
