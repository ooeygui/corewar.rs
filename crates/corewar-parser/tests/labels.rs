use corewar_core::{AddressingMode, Instruction, Modifier, Opcode};
use corewar_parser::parse_warrior;

#[test]
fn resolves_simple_label_offsets() {
    let warrior =
        parse_warrior("start MOV bomb, target\nbomb DAT #0, #0\ntarget DAT #0, #0\n").unwrap();

    assert_eq!(
        warrior.instructions[0],
        Instruction::new(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::Direct,
            1,
            AddressingMode::Direct,
            2,
        )
    );
}

#[test]
fn resolves_forward_references() {
    let warrior = parse_warrior("JMP target\nDAT #0, #0\ntarget DAT #0, #0\n").unwrap();

    assert_eq!(
        warrior.instructions[0],
        Instruction::new(
            Opcode::JMP,
            Modifier::B,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            0,
        )
    );
}

#[test]
fn resolves_labels_on_same_line_as_instruction() {
    let warrior = parse_warrior("loop MOV 0, 1\nJMP loop\n").unwrap();

    assert_eq!(warrior.instructions[1].a_value, -1);
}

#[test]
fn resolves_labels_on_their_own_line() {
    let warrior = parse_warrior("start:\n    MOV 0, 1\n    JMP start\n").unwrap();

    assert_eq!(warrior.instructions.len(), 2);
    assert_eq!(warrior.instructions[1].a_value, -1);
}
