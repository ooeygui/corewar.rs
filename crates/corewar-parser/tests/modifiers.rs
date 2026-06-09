use corewar_core::{AddressingMode, Modifier};
use corewar_parser::parse_warrior;

#[test]
fn parses_explicit_modifiers() {
    let warrior = parse_warrior(
        "MOV.A 0, 1\nMOV.B 0, 1\nADD.AB #1, 2\nMOV.BA 0, 1\nDAT.F #0, #0\nMOV.X 0, 1\nMOV.I 0, 1\n",
    )
    .unwrap();

    let modifiers: Vec<_> = warrior
        .instructions
        .iter()
        .map(|instruction| instruction.modifier)
        .collect();

    assert_eq!(
        modifiers,
        vec![
            Modifier::A,
            Modifier::B,
            Modifier::AB,
            Modifier::BA,
            Modifier::F,
            Modifier::X,
            Modifier::I,
        ]
    );
}

#[test]
fn infers_default_modifiers() {
    let warrior = parse_warrior(
        "MOV #1, 2\nMOV 1, 2\nMOV 1, #2\nADD 1, #2\nADD 1, 2\nSLT #1, 2\nSLT 1, 2\nJMP 1\nDAT #0, #0\n",
    )
    .unwrap();

    let modifiers: Vec<_> = warrior
        .instructions
        .iter()
        .map(|instruction| instruction.modifier)
        .collect();

    assert_eq!(
        modifiers,
        vec![
            Modifier::AB,
            Modifier::I,
            Modifier::B,
            Modifier::B,
            Modifier::F,
            Modifier::AB,
            Modifier::B,
            Modifier::B,
            Modifier::F,
        ]
    );

    assert_eq!(warrior.instructions[0].a_mode, AddressingMode::Immediate);
}
