mod common;

use common::{
    dwarf_program, executor_with_actor, imp_program, place_warrior, step_n, test_config, warrior,
};
use corewar_core::{CoreEvent, TimedEvent, Warrior};
use corewar_vm::{battle::BattleResult, Battle, Executor};

fn battle_config() -> corewar_vm::VmConfig {
    let mut config = test_config();
    config.core_size = 40;
    config.max_cycles = 80;
    config.max_length = 8;
    config.min_distance = 5;
    config.seed = 3;
    config
}

fn imp_warrior(name: &str) -> Warrior {
    warrior(name, imp_program())
}

fn dwarf_warrior() -> Warrior {
    warrior("Dwarf", dwarf_program())
}

#[test]
fn imp_fills_core_without_dying() {
    let imp = imp_program();
    let mut executor = executor_with_actor(&imp);

    step_n(&mut executor, 6);

    assert_eq!(executor.process_count(1), Some(1));
    assert_eq!(executor.living_warrior_ids(), vec![1, 2]);
    for address in 1..=6 {
        assert_eq!(*executor.core.read(address), imp[0]);
    }
    assert!(!executor
        .event_bus
        .drain()
        .iter()
        .any(|event| matches!(event.event, CoreEvent::ProcessKilled { warrior_id: 1, .. })));
}

#[test]
fn dwarf_bombs_the_expected_offsets() {
    let mut executor = Executor::new(test_config());
    place_warrior(&mut executor, 1, 0, &dwarf_program());
    place_warrior(&mut executor, 2, 16, &[common::jmp(0)]);

    step_n(&mut executor, 5);

    assert_eq!(*executor.core.read(7), common::dat_immediate(0, 4));
    assert_eq!(*executor.core.read(11), common::dat_immediate(0, 8));
}

#[test]
fn two_imps_facing_each_other_draw() {
    let mut battle = Battle::new(battle_config());
    battle.add_warrior(imp_warrior("Imp A"));
    battle.add_warrior(imp_warrior("Imp B"));

    match battle.run() {
        BattleResult::Draw { survivor_ids } => assert_eq!(survivor_ids.len(), 2),
        BattleResult::Win { winner_id } => panic!("expected draw, got win for {winner_id}"),
    }
}

#[test]
fn dwarf_eventually_hits_imp_with_fixed_seed() {
    let mut config = battle_config();
    config.core_size = 20;
    config.max_cycles = 200;
    config.min_distance = 4;
    config.seed = 1;

    let mut battle = Battle::new(config);
    battle.add_warrior(dwarf_warrior());
    battle.add_warrior(imp_warrior("Imp"));

    match battle.run() {
        BattleResult::Win { winner_id } => assert_eq!(winner_id, 1),
        BattleResult::Draw { survivor_ids } => {
            panic!("expected dwarf to win, survivors were {survivor_ids:?}")
        }
    }
}
