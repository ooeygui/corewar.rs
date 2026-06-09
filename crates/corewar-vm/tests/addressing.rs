mod common;

use common::{dat, executor_with_actor, instruction};
use corewar_core::{AddressingMode, Modifier, Opcode};

#[test]
fn immediate_mode_uses_the_literal_value() {
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            1,
        ),
        dat(0, 0),
    ]);

    assert!(executor.step());
    assert_eq!(executor.core.read(1).b_value, 7);
}

#[test]
fn direct_mode_resolves_relative_to_the_program_counter() {
    let source = instruction(
        Opcode::SPL,
        Modifier::X,
        AddressingMode::IndirectA,
        4,
        AddressingMode::PostIncIndirectB,
        6,
    );
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::Direct,
            1,
            AddressingMode::Direct,
            2,
        ),
        source,
        dat(0, 0),
    ]);

    assert!(executor.step());
    assert_eq!(*executor.core.read(2), source);
}

#[test]
fn b_indirect_mode_follows_the_b_field() {
    let source = instruction(
        Opcode::NOP,
        Modifier::F,
        AddressingMode::IndirectA,
        3,
        AddressingMode::PostIncIndirectB,
        4,
    );
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::IndirectB,
            1,
            AddressingMode::Direct,
            4,
        ),
        dat(0, 2),
        dat(0, 0),
        source,
        dat(0, 0),
    ]);

    assert!(executor.step());
    assert_eq!(*executor.core.read(4), source);
}

#[test]
fn a_indirect_mode_follows_the_a_field() {
    let source = instruction(
        Opcode::NOP,
        Modifier::F,
        AddressingMode::PostIncIndirectA,
        5,
        AddressingMode::IndirectB,
        6,
    );
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::IndirectA,
            1,
            AddressingMode::Direct,
            4,
        ),
        dat(2, 0),
        dat(0, 0),
        source,
        dat(0, 0),
    ]);

    assert!(executor.step());
    assert_eq!(*executor.core.read(4), source);
}

#[test]
fn predecrement_b_mode_updates_before_resolving() {
    let expected = instruction(
        Opcode::NOP,
        Modifier::F,
        AddressingMode::Direct,
        7,
        AddressingMode::Direct,
        8,
    );
    let decoy = instruction(
        Opcode::NOP,
        Modifier::F,
        AddressingMode::Direct,
        1,
        AddressingMode::Direct,
        2,
    );
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::PreDecIndirectB,
            1,
            AddressingMode::Direct,
            5,
        ),
        dat(0, 3),
        dat(0, 0),
        expected,
        decoy,
        dat(0, 0),
    ]);

    assert!(executor.step());
    assert_eq!(executor.core.read(1).b_value, 2);
    assert_eq!(*executor.core.read(5), expected);
}

#[test]
fn predecrement_a_mode_updates_before_resolving() {
    let expected = instruction(
        Opcode::NOP,
        Modifier::F,
        AddressingMode::Direct,
        7,
        AddressingMode::Direct,
        8,
    );
    let decoy = instruction(
        Opcode::NOP,
        Modifier::F,
        AddressingMode::Direct,
        1,
        AddressingMode::Direct,
        2,
    );
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::PreDecIndirectA,
            1,
            AddressingMode::Direct,
            5,
        ),
        dat(3, 0),
        dat(0, 0),
        expected,
        decoy,
        dat(0, 0),
    ]);

    assert!(executor.step());
    assert_eq!(executor.core.read(1).a_value, 2);
    assert_eq!(*executor.core.read(5), expected);
}

#[test]
fn postincrement_b_mode_updates_after_resolving() {
    let expected = instruction(
        Opcode::NOP,
        Modifier::F,
        AddressingMode::Direct,
        7,
        AddressingMode::Direct,
        8,
    );
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::PostIncIndirectB,
            1,
            AddressingMode::Direct,
            4,
        ),
        dat(0, 2),
        dat(0, 0),
        expected,
        dat(0, 0),
    ]);

    assert!(executor.step());
    assert_eq!(executor.core.read(1).b_value, 3);
    assert_eq!(*executor.core.read(4), expected);
}

#[test]
fn postincrement_a_mode_updates_after_resolving() {
    let expected = instruction(
        Opcode::NOP,
        Modifier::F,
        AddressingMode::Direct,
        7,
        AddressingMode::Direct,
        8,
    );
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::PostIncIndirectA,
            1,
            AddressingMode::Direct,
            4,
        ),
        dat(2, 0),
        dat(0, 0),
        expected,
        dat(0, 0),
    ]);

    assert!(executor.step());
    assert_eq!(executor.core.read(1).a_value, 3);
    assert_eq!(*executor.core.read(4), expected);
}

#[test]
fn mixed_addressing_modes_work_together_in_one_instruction() {
    let source = instruction(
        Opcode::SPL,
        Modifier::X,
        AddressingMode::IndirectA,
        8,
        AddressingMode::PostIncIndirectB,
        9,
    );
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::PreDecIndirectB,
            1,
            AddressingMode::PostIncIndirectB,
            2,
        ),
        dat(0, 3),
        dat(0, 2),
        source,
        dat(0, 0),
    ]);

    assert!(executor.step());
    assert_eq!(executor.core.read(1).b_value, 2);
    assert_eq!(executor.core.read(2).b_value, 3);
    assert_eq!(*executor.core.read(4), source);
}
