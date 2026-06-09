use corewar_parser::{parse_warrior, ParseError};

#[test]
fn invalid_opcode_produces_error() {
    let error = parse_warrior("label FOO 0, 1\n").unwrap_err();

    assert!(matches!(
        error,
        ParseError::InvalidOpcode { opcode, line }
            if opcode == "FOO" && line == 1
    ));
}

#[test]
fn undefined_label_produces_error() {
    let error = parse_warrior("MOV missing, 0\n").unwrap_err();

    assert!(matches!(
        error,
        ParseError::UndefinedLabel { label, line }
            if label == "MISSING" && line == 1
    ));
}

#[test]
fn duplicate_label_produces_error() {
    let error = parse_warrior("start MOV 0, 1\nstart DAT #0, #0\n").unwrap_err();

    assert!(matches!(
        error,
        ParseError::DuplicateLabel { label, line }
            if label == "START" && line == 2
    ));
}

#[test]
fn empty_input_produces_empty_warrior() {
    let warrior = parse_warrior("").unwrap();

    assert!(warrior.instructions.is_empty());
    assert_eq!(warrior.start_offset, 0);
    assert_eq!(warrior.name, "unnamed");
    assert_eq!(warrior.author, "");
}
