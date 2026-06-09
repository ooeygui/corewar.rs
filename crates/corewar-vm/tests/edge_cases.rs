mod common;

use common::{dat, imp_program, instruction, jmp, place_warrior, test_config, warrior};
use corewar_core::{AddressingMode, Modifier, Opcode};
use corewar_vm::{battle::BattleResult, Battle, Executor};

#[test]
fn division_and_modulo_by_zero_kill_the_process() {
    for opcode in [Opcode::DIV, Opcode::MOD] {
        let mut executor = Executor::new(test_config());
        place_warrior(
            &mut executor,
            1,
            0,
            &[
                instruction(
                    opcode,
                    Modifier::AB,
                    AddressingMode::Immediate,
                    0,
                    AddressingMode::Direct,
                    1,
                ),
                dat(3, 9),
            ],
        );
        place_warrior(&mut executor, 2, 16, &[jmp(0)]);

        assert!(
            !executor.step(),
            "{opcode:?} should kill the current process"
        );
        assert_eq!(executor.living_warrior_ids(), vec![2]);
        assert_eq!(*executor.core.read(1), dat(3, 9));
    }
}

#[test]
fn core_addresses_wrap_forward_past_the_end_of_memory() {
    let mut config = test_config();
    config.core_size = 8;
    let mut executor = Executor::new(config.clone());

    place_warrior(
        &mut executor,
        1,
        config.core_size - 1,
        &[instruction(
            Opcode::MOV,
            Modifier::AB,
            AddressingMode::Immediate,
            7,
            AddressingMode::Direct,
            1,
        )],
    );
    place_warrior(&mut executor, 2, 3, &[jmp(0)]);

    assert!(executor.step());
    assert_eq!(executor.core.read(0).b_value, 7);
}

#[test]
fn hitting_the_cycle_limit_produces_a_draw() {
    let mut config = test_config();
    config.max_cycles = 3;
    config.seed = 5;

    let mut battle = Battle::new(config);
    battle.add_warrior(warrior("Imp A", imp_program()));
    battle.add_warrior(warrior("Imp B", imp_program()));

    match battle.run() {
        BattleResult::Draw { survivor_ids } => assert_eq!(survivor_ids.len(), 2),
        BattleResult::Win { winner_id } => panic!("expected draw, got win for {winner_id}"),
    }
}

#[test]
fn empty_warrior_is_eliminated_cleanly() {
    let mut config = test_config();
    config.core_size = 24;
    config.max_length = 4;
    config.min_distance = 3;
    config.seed = 2;

    let mut battle = Battle::new(config);
    battle.add_warrior(warrior("Empty", Vec::new()));
    battle.add_warrior(warrior("Looper", vec![jmp(0)]));

    match battle.run() {
        BattleResult::Win { winner_id } => assert_eq!(winner_id, 2),
        BattleResult::Draw { survivor_ids } => {
            panic!("expected non-empty warrior to win, survivors were {survivor_ids:?}")
        }
    }
}
