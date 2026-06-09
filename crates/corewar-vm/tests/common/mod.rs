use corewar_core::{AddressingMode, Instruction, Modifier, Opcode, Warrior};
use corewar_vm::{Executor, VmConfig};

pub const ACTOR_ID: u32 = 1;
pub const DUMMY_ID: u32 = 2;

pub fn test_config() -> VmConfig {
    VmConfig {
        core_size: 32,
        max_cycles: 64,
        max_processes: 16,
        max_length: 16,
        min_distance: 4,
        seed: 7,
    }
}

pub fn instruction(
    opcode: Opcode,
    modifier: Modifier,
    a_mode: AddressingMode,
    a_value: i32,
    b_mode: AddressingMode,
    b_value: i32,
) -> Instruction {
    Instruction::new(opcode, modifier, a_mode, a_value, b_mode, b_value)
}

pub fn dat(a_value: i32, b_value: i32) -> Instruction {
    instruction(
        Opcode::DAT,
        Modifier::F,
        AddressingMode::Direct,
        a_value,
        AddressingMode::Direct,
        b_value,
    )
}

pub fn dat_immediate(a_value: i32, b_value: i32) -> Instruction {
    instruction(
        Opcode::DAT,
        Modifier::F,
        AddressingMode::Immediate,
        a_value,
        AddressingMode::Immediate,
        b_value,
    )
}

pub fn nop() -> Instruction {
    instruction(
        Opcode::NOP,
        Modifier::F,
        AddressingMode::Direct,
        0,
        AddressingMode::Direct,
        0,
    )
}

pub fn jmp(offset: i32) -> Instruction {
    instruction(
        Opcode::JMP,
        Modifier::B,
        AddressingMode::Direct,
        offset,
        AddressingMode::Direct,
        0,
    )
}

pub fn place_warrior(
    executor: &mut Executor,
    warrior_id: u32,
    address: usize,
    instructions: &[Instruction],
) {
    for (offset, instruction) in instructions.iter().copied().enumerate() {
        executor
            .core
            .write_with_owner(address + offset, instruction, warrior_id);
    }
    executor.add_warrior(warrior_id, address);
}

pub fn executor_with_actor(program: &[Instruction]) -> Executor {
    executor_with_actor_and_config(test_config(), 0, program)
}

pub fn executor_with_actor_and_config(
    config: VmConfig,
    actor_address: usize,
    program: &[Instruction],
) -> Executor {
    let dummy_address = (actor_address + config.core_size / 2) % config.core_size;
    let mut executor = Executor::new(config);
    place_warrior(&mut executor, ACTOR_ID, actor_address, program);
    place_warrior(&mut executor, DUMMY_ID, dummy_address, &[jmp(0)]);
    executor
}

pub fn step_n(executor: &mut Executor, steps: usize) {
    for _ in 0..steps {
        executor.step();
    }
}

pub fn warrior(name: &str, instructions: Vec<Instruction>) -> Warrior {
    Warrior::new(name, instructions)
}

pub fn imp_program() -> Vec<Instruction> {
    vec![instruction(
        Opcode::MOV,
        Modifier::I,
        AddressingMode::Direct,
        0,
        AddressingMode::Direct,
        1,
    )]
}

pub fn dwarf_program() -> Vec<Instruction> {
    vec![
        instruction(
            Opcode::ADD,
            Modifier::AB,
            AddressingMode::Immediate,
            4,
            AddressingMode::Direct,
            3,
        ),
        instruction(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::Direct,
            2,
            AddressingMode::IndirectB,
            2,
        ),
        instruction(
            Opcode::JMP,
            Modifier::B,
            AddressingMode::Direct,
            -2,
            AddressingMode::Direct,
            0,
        ),
        dat_immediate(0, 0),
    ]
}
