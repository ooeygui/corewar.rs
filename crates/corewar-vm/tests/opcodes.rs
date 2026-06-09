mod common;

use common::{executor_with_actor, instruction, step_n};
use corewar_core::{AddressingMode, CoreEvent, Instruction, Modifier, Opcode, TimedEvent};

fn arithmetic_result(opcode: Opcode, destination: i32, source: i32, core_size: usize) -> i32 {
    let value = match opcode {
        Opcode::ADD => i64::from(destination) + i64::from(source),
        Opcode::SUB => i64::from(destination) - i64::from(source),
        Opcode::MUL => i64::from(destination) * i64::from(source),
        Opcode::DIV => i64::from(destination) / i64::from(source),
        Opcode::MOD => i64::from(destination) % i64::from(source),
        other => panic!("unexpected arithmetic opcode: {other:?}"),
    };

    value.rem_euclid(core_size as i64) as i32
}

fn expected_fields(
    modifier: Modifier,
    source: Instruction,
    destination: Instruction,
    opcode: Opcode,
    core_size: usize,
) -> (i32, i32) {
    match modifier {
        Modifier::A => (
            arithmetic_result(opcode, destination.a_value, source.a_value, core_size),
            destination.b_value,
        ),
        Modifier::B => (
            destination.a_value,
            arithmetic_result(opcode, destination.b_value, source.b_value, core_size),
        ),
        Modifier::AB => (
            destination.a_value,
            arithmetic_result(opcode, destination.b_value, source.a_value, core_size),
        ),
        Modifier::BA => (
            arithmetic_result(opcode, destination.a_value, source.b_value, core_size),
            destination.b_value,
        ),
        Modifier::F | Modifier::I => (
            arithmetic_result(opcode, destination.a_value, source.a_value, core_size),
            arithmetic_result(opcode, destination.b_value, source.b_value, core_size),
        ),
        Modifier::X => (
            arithmetic_result(opcode, destination.a_value, source.b_value, core_size),
            arithmetic_result(opcode, destination.b_value, source.a_value, core_size),
        ),
    }
}

#[test]
fn dat_process_dies_immediately() {
    let mut executor = executor_with_actor(&[instruction(
        Opcode::DAT,
        Modifier::F,
        AddressingMode::Direct,
        0,
        AddressingMode::Direct,
        0,
    )]);

    assert!(!executor.step());
    assert_eq!(executor.living_warrior_ids(), vec![2]);
    assert!(executor.event_bus.drain().contains(&TimedEvent {
        cycle: 0,
        event: CoreEvent::ProcessKilled {
            warrior_id: 1,
            address: 0,
        },
    }));
}

#[test]
fn mov_modifiers_copy_expected_fields() {
    let source = instruction(
        Opcode::SPL,
        Modifier::X,
        AddressingMode::IndirectA,
        7,
        AddressingMode::PostIncIndirectB,
        9,
    );
    let destination = instruction(
        Opcode::DAT,
        Modifier::F,
        AddressingMode::Direct,
        1,
        AddressingMode::Direct,
        2,
    );

    let cases = [
        (
            Modifier::A,
            instruction(
                Opcode::DAT,
                Modifier::F,
                AddressingMode::IndirectA,
                7,
                AddressingMode::Direct,
                2,
            ),
        ),
        (
            Modifier::B,
            instruction(
                Opcode::DAT,
                Modifier::F,
                AddressingMode::Direct,
                1,
                AddressingMode::PostIncIndirectB,
                9,
            ),
        ),
        (
            Modifier::AB,
            instruction(
                Opcode::DAT,
                Modifier::F,
                AddressingMode::Direct,
                1,
                AddressingMode::IndirectA,
                7,
            ),
        ),
        (
            Modifier::BA,
            instruction(
                Opcode::DAT,
                Modifier::F,
                AddressingMode::PostIncIndirectB,
                9,
                AddressingMode::Direct,
                2,
            ),
        ),
        (
            Modifier::F,
            instruction(
                Opcode::DAT,
                Modifier::F,
                AddressingMode::IndirectA,
                7,
                AddressingMode::PostIncIndirectB,
                9,
            ),
        ),
        (
            Modifier::X,
            instruction(
                Opcode::DAT,
                Modifier::F,
                AddressingMode::PostIncIndirectB,
                9,
                AddressingMode::IndirectA,
                7,
            ),
        ),
        (Modifier::I, source),
    ];

    for (modifier, expected) in cases {
        let mut executor = executor_with_actor(&[
            instruction(
                Opcode::MOV,
                modifier,
                AddressingMode::Direct,
                1,
                AddressingMode::Direct,
                2,
            ),
            source,
            destination,
        ]);

        assert!(
            executor.step(),
            "MOV.{modifier:?} should keep the battle running"
        );
        assert_eq!(
            *executor.core.read(2),
            expected,
            "failed for MOV.{modifier:?}"
        );
    }
}

#[test]
fn arithmetic_opcodes_apply_all_modifiers() {
    let source = instruction(
        Opcode::DAT,
        Modifier::F,
        AddressingMode::Direct,
        2,
        AddressingMode::Direct,
        3,
    );
    let destination = instruction(
        Opcode::DAT,
        Modifier::F,
        AddressingMode::IndirectA,
        11,
        AddressingMode::PostIncIndirectB,
        13,
    );
    let opcodes = [
        Opcode::ADD,
        Opcode::SUB,
        Opcode::MUL,
        Opcode::DIV,
        Opcode::MOD,
    ];
    let modifiers = [
        Modifier::A,
        Modifier::B,
        Modifier::AB,
        Modifier::BA,
        Modifier::F,
        Modifier::X,
        Modifier::I,
    ];

    for opcode in opcodes {
        for modifier in modifiers {
            let mut executor = executor_with_actor(&[
                instruction(
                    opcode,
                    modifier,
                    AddressingMode::Direct,
                    1,
                    AddressingMode::Direct,
                    2,
                ),
                source,
                destination,
            ]);

            assert!(
                executor.step(),
                "{opcode:?}.{modifier:?} should keep running"
            );

            let updated = *executor.core.read(2);
            let (expected_a, expected_b) =
                expected_fields(modifier, source, destination, opcode, executor.core.size());
            assert_eq!((updated.a_value, updated.b_value), (expected_a, expected_b));
            assert_eq!(
                updated.opcode, destination.opcode,
                "opcode changed for {opcode:?}.{modifier:?}"
            );
            assert_eq!(
                updated.a_mode, destination.a_mode,
                "A-mode changed for {opcode:?}.{modifier:?}"
            );
            assert_eq!(
                updated.b_mode, destination.b_mode,
                "B-mode changed for {opcode:?}.{modifier:?}"
            );
        }
    }
}

#[test]
fn jmp_changes_the_program_counter() {
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::JMP,
            Modifier::B,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            0,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            1,
            AddressingMode::Direct,
            5,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            5,
        ),
    ]);

    step_n(&mut executor, 2);

    assert_eq!(executor.core.read(5).b_value, 7);
}

#[test]
fn jmz_jumps_on_zero_and_falls_through_otherwise() {
    let mut jump_executor = executor_with_actor(&[
        instruction(
            Opcode::JMZ,
            Modifier::B,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            4,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            1,
            AddressingMode::Direct,
            5,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            5,
        ),
        instruction(
            Opcode::NOP,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            0,
        ),
        instruction(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            0,
        ),
    ]);
    step_n(&mut jump_executor, 2);
    assert_eq!(jump_executor.core.read(5).b_value, 7);

    let mut fallthrough_executor = executor_with_actor(&[
        instruction(
            Opcode::JMZ,
            Modifier::B,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            4,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            1,
            AddressingMode::Direct,
            5,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            5,
        ),
        instruction(
            Opcode::NOP,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            0,
        ),
        instruction(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            3,
        ),
    ]);
    step_n(&mut fallthrough_executor, 2);
    assert_eq!(fallthrough_executor.core.read(5).b_value, 1);
}

#[test]
fn jmn_jumps_only_when_non_zero() {
    let mut jump_executor = executor_with_actor(&[
        instruction(
            Opcode::JMN,
            Modifier::B,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            4,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            1,
            AddressingMode::Direct,
            5,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            5,
        ),
        instruction(
            Opcode::NOP,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            0,
        ),
        instruction(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            3,
        ),
    ]);
    step_n(&mut jump_executor, 2);
    assert_eq!(jump_executor.core.read(5).b_value, 7);

    let mut fallthrough_executor = executor_with_actor(&[
        instruction(
            Opcode::JMN,
            Modifier::B,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            4,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            1,
            AddressingMode::Direct,
            5,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            5,
        ),
        instruction(
            Opcode::NOP,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            0,
        ),
        instruction(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            0,
        ),
    ]);
    step_n(&mut fallthrough_executor, 2);
    assert_eq!(fallthrough_executor.core.read(5).b_value, 1);
}

#[test]
fn djn_decrements_then_jumps_if_non_zero() {
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::DJN,
            Modifier::B,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            4,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            1,
            AddressingMode::Direct,
            5,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            5,
        ),
        instruction(
            Opcode::NOP,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            0,
        ),
        instruction(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            2,
        ),
    ]);

    assert!(executor.step());
    assert_eq!(executor.core.read(4).b_value, 1);
    assert!(executor.step());
    assert_eq!(executor.core.read(5).b_value, 7);
}

#[test]
fn seq_and_sne_skip_the_next_instruction_when_their_condition_matches() {
    let mut seq_executor = executor_with_actor(&[
        instruction(
            Opcode::SEQ,
            Modifier::I,
            AddressingMode::Direct,
            3,
            AddressingMode::Direct,
            4,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            1,
            AddressingMode::Direct,
            6,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            6,
        ),
        instruction(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            3,
        ),
        instruction(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            3,
        ),
    ]);
    step_n(&mut seq_executor, 2);
    assert_eq!(seq_executor.core.read(6).b_value, 7);

    let mut sne_executor = executor_with_actor(&[
        instruction(
            Opcode::SNE,
            Modifier::I,
            AddressingMode::Direct,
            3,
            AddressingMode::Direct,
            4,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            1,
            AddressingMode::Direct,
            6,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            6,
        ),
        instruction(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            3,
        ),
        instruction(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            9,
            AddressingMode::Direct,
            3,
        ),
    ]);
    step_n(&mut sne_executor, 2);
    assert_eq!(sne_executor.core.read(6).b_value, 7);
}

#[test]
fn slt_skips_when_left_operand_is_less_than_right_operand() {
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::SLT,
            Modifier::AB,
            AddressingMode::Direct,
            3,
            AddressingMode::Direct,
            4,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            1,
            AddressingMode::Direct,
            6,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            6,
        ),
        instruction(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            9,
        ),
        instruction(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            5,
        ),
    ]);

    step_n(&mut executor, 2);

    assert_eq!(executor.core.read(6).b_value, 7);
}

#[test]
fn spl_creates_a_child_process_and_both_continue() {
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::SPL,
            Modifier::B,
            AddressingMode::Direct,
            2,
            AddressingMode::Direct,
            0,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            1,
            AddressingMode::Direct,
            6,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            7,
        ),
    ]);

    assert!(executor.step());
    assert_eq!(executor.process_count(1), Some(2));

    assert!(executor.step());
    assert_eq!(executor.core.read(6).b_value, 1);

    assert!(executor.step());
    assert_eq!(executor.core.read(7).b_value, 7);
}

#[test]
fn nop_advances_to_the_next_instruction_without_side_effects() {
    let mut executor = executor_with_actor(&[
        instruction(
            Opcode::NOP,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            0,
        ),
        instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            5,
        ),
    ]);

    let before = executor.core.snapshot();
    assert!(executor.step());
    assert_eq!(executor.core.snapshot(), before);

    assert!(executor.step());
    assert_eq!(executor.core.read(5).b_value, 7);
}
