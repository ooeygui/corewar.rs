mod common;

use common::{dat, instruction, jmp, place_warrior, test_config};
use corewar_core::{AddressingMode, CoreEvent, Modifier, Opcode, TimedEvent};
use corewar_vm::Executor;

#[test]
fn spl_stops_creating_processes_at_the_vm_limit() {
    let mut config = test_config();
    config.max_processes = 4;
    config.max_cycles = 32;

    let mut executor = Executor::new(config);
    place_warrior(
        &mut executor,
        1,
        0,
        &[
            instruction(
                Opcode::SPL,
                Modifier::B,
                AddressingMode::Direct,
                2,
                AddressingMode::Direct,
                0,
            ),
            jmp(-1),
            jmp(-2),
        ],
    );
    place_warrior(&mut executor, 2, 16, &[jmp(0)]);

    for _ in 0..8 {
        assert!(executor.step());
    }

    assert_eq!(executor.process_count(1), Some(4));
}

#[test]
fn process_queue_runs_in_round_robin_order() {
    let mut executor = Executor::new(test_config());
    place_warrior(
        &mut executor,
        1,
        0,
        &[
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
                5,
            ),
            instruction(
                Opcode::MOV,
                Modifier::AB,
                AddressingMode::Immediate,
                2,
                AddressingMode::Direct,
                5,
            ),
        ],
    );
    place_warrior(&mut executor, 2, 16, &[jmp(0)]);

    assert!(executor.step());
    assert_eq!(executor.process_count(1), Some(2));

    assert!(executor.step());
    assert_eq!(executor.core.read(5).b_value, 1);

    assert!(executor.step());
    assert_eq!(executor.core.read(5).b_value, 2);
}

#[test]
fn warrior_is_eliminated_when_its_last_process_dies() {
    let mut executor = Executor::new(test_config());
    place_warrior(&mut executor, 1, 0, &[dat(0, 0)]);
    place_warrior(&mut executor, 2, 16, &[jmp(0)]);

    assert!(!executor.step());
    assert_eq!(executor.living_warrior_ids(), vec![2]);
    assert!(executor.event_bus.drain().contains(&TimedEvent {
        cycle: 0,
        event: CoreEvent::WarriorEliminated { warrior_id: 1 },
    }));
}

#[test]
fn multiple_warriors_are_eliminated_in_execution_order() {
    let mut executor = Executor::new(test_config());
    place_warrior(&mut executor, 1, 0, &[dat(0, 0)]);
    place_warrior(&mut executor, 2, 8, &[dat(0, 0)]);
    place_warrior(&mut executor, 3, 16, &[jmp(0)]);

    assert!(!executor.step());

    let elimination_order: Vec<u32> = executor
        .event_bus
        .drain()
        .into_iter()
        .filter_map(|event| match event.event {
            CoreEvent::WarriorEliminated { warrior_id } => Some(warrior_id),
            _ => None,
        })
        .collect();

    assert_eq!(elimination_order, vec![1, 2]);
    assert_eq!(executor.living_warrior_ids(), vec![3]);
}
