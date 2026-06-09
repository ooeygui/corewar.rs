//! Instruction execution engine.

use corewar_core::{
    AddressingMode, Core, CoreEvent, EventBus, Instruction, Modifier, Opcode, ProcessQueue,
};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::{
    address::{resolve_a_with_events, resolve_b_with_events},
    config::VmConfig,
};

#[derive(Debug, Clone)]
struct WarriorState {
    id: u32,
    processes: ProcessQueue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OperandField {
    mode: AddressingMode,
    value: i32,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedOperand {
    instruction: Instruction,
    literal: Option<i32>,
}

impl ResolvedOperand {
    fn from_a(core: &Core, instruction: &Instruction, address: usize) -> Self {
        let literal =
            (instruction.a_mode == AddressingMode::Immediate).then_some(instruction.a_value);
        let instruction = literal
            .map(immediate_instruction)
            .unwrap_or_else(|| *core.read(address));
        Self {
            instruction,
            literal,
        }
    }

    fn from_b(core: &Core, instruction: &Instruction, address: usize) -> Self {
        let literal =
            (instruction.b_mode == AddressingMode::Immediate).then_some(instruction.b_value);
        let instruction = literal
            .map(immediate_instruction)
            .unwrap_or_else(|| *core.read(address));
        Self {
            instruction,
            literal,
        }
    }

    fn instruction(self) -> Instruction {
        self.instruction
    }

    fn a_field(self) -> OperandField {
        OperandField {
            mode: self.instruction.a_mode,
            value: self.instruction.a_value,
        }
    }

    fn b_field(self) -> OperandField {
        OperandField {
            mode: self.instruction.b_mode,
            value: self.instruction.b_value,
        }
    }

    fn a_value(self) -> i32 {
        self.literal.unwrap_or(self.instruction.a_value)
    }

    fn b_value(self) -> i32 {
        self.literal.unwrap_or(self.instruction.b_value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecOutcome {
    Continue {
        next_pc: usize,
        child: Option<usize>,
    },
    Die,
}

/// The execution engine that steps through processes.
pub struct Executor {
    pub core: Core,
    pub event_bus: EventBus,
    pub cycle: u64,
    config: VmConfig,
    warriors: Vec<WarriorState>,
}

impl Executor {
    pub fn new(config: VmConfig) -> Self {
        let core = Core::new(config.core_size);
        Self {
            core,
            event_bus: EventBus::new(),
            cycle: 0,
            config,
            warriors: Vec::new(),
        }
    }

    pub fn config(&self) -> &VmConfig {
        &self.config
    }

    pub fn add_warrior(&mut self, warrior_id: u32, entry_pc: usize) {
        let mut processes = ProcessQueue::new();
        processes.push(entry_pc % self.core.size());
        self.warriors.push(WarriorState {
            id: warrior_id,
            processes,
        });
    }

    pub fn living_warrior_ids(&self) -> Vec<u32> {
        self.warriors.iter().map(|warrior| warrior.id).collect()
    }

    pub fn process_count(&self, warrior_id: u32) -> Option<usize> {
        self.warriors
            .iter()
            .find(|warrior| warrior.id == warrior_id)
            .map(|warrior| warrior.processes.len())
    }

    /// Execute one cycle (one instruction per living warrior).
    pub fn step(&mut self) -> bool {
        if self.cycle >= self.config.max_cycles || self.warriors.len() <= 1 {
            return false;
        }

        let warrior_count = self.warriors.len();
        self.event_bus.set_cycle(self.cycle);

        for index in 0..warrior_count {
            let (warrior_id, pc, can_split) = {
                let warrior = &mut self.warriors[index];
                let Some(pc) = warrior.processes.pop() else {
                    continue;
                };
                let can_split = warrior.processes.len() + 2 <= self.config.max_processes;
                (warrior.id, pc, can_split)
            };

            let outcome = self.execute_process(warrior_id, pc, can_split);
            let warrior = &mut self.warriors[index];

            match outcome {
                ExecOutcome::Continue { next_pc, child } => {
                    warrior.processes.push(next_pc);
                    if let Some(child_pc) = child {
                        warrior.processes.push(child_pc);
                    }
                }
                ExecOutcome::Die => {
                    self.event_bus.emit(CoreEvent::ProcessKilled {
                        warrior_id,
                        address: pc,
                    });
                }
            }

            if warrior.processes.is_empty() {
                self.event_bus
                    .emit(CoreEvent::WarriorEliminated { warrior_id });
            }
        }

        self.warriors
            .retain(|warrior| !warrior.processes.is_empty());
        self.cycle += 1;
        self.event_bus.set_cycle(self.cycle);
        self.event_bus.emit(CoreEvent::CycleComplete);

        self.cycle < self.config.max_cycles && self.warriors.len() > 1
    }

    /// Run until battle completion.
    pub fn run(&mut self) {
        while self.step() {}
    }

    fn execute_process(&mut self, warrior_id: u32, pc: usize, can_split: bool) -> ExecOutcome {
        let instruction = *self.core.read(pc);
        self.event_bus.emit(CoreEvent::Execute {
            address: pc,
            warrior_id,
        });
        let next_pc = self.core.normalize(pc as i32 + 1);
        let a_address = resolve_a_with_events(
            &mut self.core,
            pc,
            &instruction,
            warrior_id,
            &self.event_bus,
        );
        let b_address = resolve_b_with_events(
            &mut self.core,
            pc,
            &instruction,
            warrior_id,
            &self.event_bus,
        );
        let a_operand = ResolvedOperand::from_a(&self.core, &instruction, a_address);
        let b_operand = ResolvedOperand::from_b(&self.core, &instruction, b_address);

        match instruction.opcode {
            Opcode::DAT => ExecOutcome::Die,
            Opcode::MOV => {
                let mut destination = *self.core.read(b_address);
                apply_mov(instruction.modifier, a_operand, &mut destination);
                self.core
                    .write(b_address, destination, warrior_id, &self.event_bus);
                ExecOutcome::Continue {
                    next_pc,
                    child: None,
                }
            }
            Opcode::ADD => self.execute_arithmetic(
                warrior_id,
                instruction.modifier,
                a_operand,
                b_address,
                ArithmeticOp::Add,
                next_pc,
            ),
            Opcode::SUB => self.execute_arithmetic(
                warrior_id,
                instruction.modifier,
                a_operand,
                b_address,
                ArithmeticOp::Sub,
                next_pc,
            ),
            Opcode::MUL => self.execute_arithmetic(
                warrior_id,
                instruction.modifier,
                a_operand,
                b_address,
                ArithmeticOp::Mul,
                next_pc,
            ),
            Opcode::DIV => self.execute_arithmetic(
                warrior_id,
                instruction.modifier,
                a_operand,
                b_address,
                ArithmeticOp::Div,
                next_pc,
            ),
            Opcode::MOD => self.execute_arithmetic(
                warrior_id,
                instruction.modifier,
                a_operand,
                b_address,
                ArithmeticOp::Mod,
                next_pc,
            ),
            Opcode::JMP => ExecOutcome::Continue {
                next_pc: a_address,
                child: None,
            },
            Opcode::JMZ => ExecOutcome::Continue {
                next_pc: if modifier_values_match(
                    instruction.modifier,
                    b_operand.instruction(),
                    |value| value == 0,
                ) {
                    a_address
                } else {
                    next_pc
                },
                child: None,
            },
            Opcode::JMN => ExecOutcome::Continue {
                next_pc: if modifier_values_match(
                    instruction.modifier,
                    b_operand.instruction(),
                    |value| value != 0,
                ) {
                    a_address
                } else {
                    next_pc
                },
                child: None,
            },
            Opcode::DJN => {
                let mut destination = *self.core.read(b_address);
                decrement_fields(&self.core, instruction.modifier, &mut destination);
                self.core
                    .write(b_address, destination, warrior_id, &self.event_bus);
                ExecOutcome::Continue {
                    next_pc: if modifier_values_match(instruction.modifier, destination, |value| {
                        value != 0
                    }) {
                        a_address
                    } else {
                        next_pc
                    },
                    child: None,
                }
            }
            Opcode::SEQ => ExecOutcome::Continue {
                next_pc: if compare_operands(instruction.modifier, a_operand, b_operand) {
                    self.core.normalize(pc as i32 + 2)
                } else {
                    next_pc
                },
                child: None,
            },
            Opcode::SNE => ExecOutcome::Continue {
                next_pc: if !compare_operands(instruction.modifier, a_operand, b_operand) {
                    self.core.normalize(pc as i32 + 2)
                } else {
                    next_pc
                },
                child: None,
            },
            Opcode::SLT => ExecOutcome::Continue {
                next_pc: if less_than_operands(instruction.modifier, a_operand, b_operand) {
                    self.core.normalize(pc as i32 + 2)
                } else {
                    next_pc
                },
                child: None,
            },
            Opcode::SPL => {
                let child = can_split.then_some(a_address);
                if let Some(address) = child {
                    self.event_bus.emit(CoreEvent::ProcessCreated {
                        warrior_id,
                        address,
                    });
                }
                ExecOutcome::Continue { next_pc, child }
            }
            Opcode::NOP => ExecOutcome::Continue {
                next_pc,
                child: None,
            },
        }
    }

    fn execute_arithmetic(
        &mut self,
        warrior_id: u32,
        modifier: Modifier,
        source: ResolvedOperand,
        destination_address: usize,
        operation: ArithmeticOp,
        next_pc: usize,
    ) -> ExecOutcome {
        let mut destination = *self.core.read(destination_address);
        if apply_arithmetic(&self.core, modifier, source, &mut destination, operation).is_err() {
            return ExecOutcome::Die;
        }

        self.core.write(
            destination_address,
            destination,
            warrior_id,
            &self.event_bus,
        );
        ExecOutcome::Continue {
            next_pc,
            child: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

fn immediate_instruction(value: i32) -> Instruction {
    Instruction::new(
        Opcode::DAT,
        Modifier::F,
        AddressingMode::Immediate,
        value,
        AddressingMode::Immediate,
        value,
    )
}

fn apply_mov(modifier: Modifier, source: ResolvedOperand, destination: &mut Instruction) {
    match modifier {
        Modifier::A => set_a_field(destination, source.a_field()),
        Modifier::B => set_b_field(destination, source.b_field()),
        Modifier::AB => set_b_field(destination, source.a_field()),
        Modifier::BA => set_a_field(destination, source.b_field()),
        Modifier::F => {
            set_a_field(destination, source.a_field());
            set_b_field(destination, source.b_field());
        }
        Modifier::X => {
            set_a_field(destination, source.b_field());
            set_b_field(destination, source.a_field());
        }
        Modifier::I => *destination = source.instruction(),
    }
}

fn set_a_field(destination: &mut Instruction, field: OperandField) {
    destination.a_mode = field.mode;
    destination.a_value = field.value;
}

fn set_b_field(destination: &mut Instruction, field: OperandField) {
    destination.b_mode = field.mode;
    destination.b_value = field.value;
}

fn apply_arithmetic(
    core: &Core,
    modifier: Modifier,
    source: ResolvedOperand,
    destination: &mut Instruction,
    operation: ArithmeticOp,
) -> Result<(), ()> {
    match modifier {
        Modifier::A => apply_numeric(core, &mut destination.a_value, source.a_value(), operation)?,
        Modifier::B => apply_numeric(core, &mut destination.b_value, source.b_value(), operation)?,
        Modifier::AB => apply_numeric(core, &mut destination.b_value, source.a_value(), operation)?,
        Modifier::BA => apply_numeric(core, &mut destination.a_value, source.b_value(), operation)?,
        Modifier::F | Modifier::I => {
            apply_numeric(core, &mut destination.a_value, source.a_value(), operation)?;
            apply_numeric(core, &mut destination.b_value, source.b_value(), operation)?;
        }
        Modifier::X => {
            apply_numeric(core, &mut destination.a_value, source.b_value(), operation)?;
            apply_numeric(core, &mut destination.b_value, source.a_value(), operation)?;
        }
    }
    Ok(())
}

fn apply_numeric(
    core: &Core,
    destination: &mut i32,
    source: i32,
    operation: ArithmeticOp,
) -> Result<(), ()> {
    let result = match operation {
        ArithmeticOp::Add => i64::from(*destination) + i64::from(source),
        ArithmeticOp::Sub => i64::from(*destination) - i64::from(source),
        ArithmeticOp::Mul => i64::from(*destination) * i64::from(source),
        ArithmeticOp::Div => {
            if source == 0 {
                return Err(());
            }
            i64::from(*destination) / i64::from(source)
        }
        ArithmeticOp::Mod => {
            if source == 0 {
                return Err(());
            }
            i64::from(*destination) % i64::from(source)
        }
    };
    *destination = normalize_value(core, result);
    Ok(())
}

fn normalize_value(core: &Core, value: i64) -> i32 {
    value.rem_euclid(core.size() as i64) as i32
}

fn decrement_fields(core: &Core, modifier: Modifier, instruction: &mut Instruction) {
    match modifier {
        Modifier::A | Modifier::BA => {
            instruction.a_value = normalize_value(core, i64::from(instruction.a_value) - 1)
        }
        Modifier::B | Modifier::AB => {
            instruction.b_value = normalize_value(core, i64::from(instruction.b_value) - 1)
        }
        Modifier::F | Modifier::I | Modifier::X => {
            instruction.a_value = normalize_value(core, i64::from(instruction.a_value) - 1);
            instruction.b_value = normalize_value(core, i64::from(instruction.b_value) - 1);
        }
    }
}

fn modifier_values_match(
    modifier: Modifier,
    instruction: Instruction,
    predicate: impl Fn(i32) -> bool,
) -> bool {
    match modifier {
        Modifier::A | Modifier::BA => predicate(instruction.a_value),
        Modifier::B | Modifier::AB => predicate(instruction.b_value),
        Modifier::F | Modifier::I | Modifier::X => {
            predicate(instruction.a_value) && predicate(instruction.b_value)
        }
    }
}

fn compare_operands(
    modifier: Modifier,
    a_operand: ResolvedOperand,
    b_operand: ResolvedOperand,
) -> bool {
    match modifier {
        Modifier::A => a_operand.a_field() == b_operand.a_field(),
        Modifier::B => a_operand.b_field() == b_operand.b_field(),
        Modifier::AB => a_operand.a_field() == b_operand.b_field(),
        Modifier::BA => a_operand.b_field() == b_operand.a_field(),
        Modifier::F => {
            a_operand.a_field() == b_operand.a_field() && a_operand.b_field() == b_operand.b_field()
        }
        Modifier::X => {
            a_operand.a_field() == b_operand.b_field() && a_operand.b_field() == b_operand.a_field()
        }
        Modifier::I => a_operand.instruction() == b_operand.instruction(),
    }
}

fn less_than_operands(
    modifier: Modifier,
    a_operand: ResolvedOperand,
    b_operand: ResolvedOperand,
) -> bool {
    match modifier {
        Modifier::A => a_operand.a_value() < b_operand.a_value(),
        Modifier::B => a_operand.b_value() < b_operand.b_value(),
        Modifier::AB => a_operand.a_value() < b_operand.b_value(),
        Modifier::BA => a_operand.b_value() < b_operand.a_value(),
        Modifier::F | Modifier::I => {
            a_operand.a_value() < b_operand.a_value() && a_operand.b_value() < b_operand.b_value()
        }
        Modifier::X => {
            a_operand.a_value() < b_operand.b_value() && a_operand.b_value() < b_operand.a_value()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Executor;
    use crate::config::VmConfig;
    use corewar_core::{AddressingMode, CoreEvent, Instruction, Modifier, Opcode, TimedEvent};

    fn config() -> VmConfig {
        VmConfig {
            core_size: 32,
            max_cycles: 32,
            max_processes: 8,
            max_length: 8,
            min_distance: 4,
            seed: 7,
        }
    }

    fn place(
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

    #[test]
    fn imp_processes_keep_running() {
        let mut executor = Executor::new(config());
        let imp = Instruction::new(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            1,
        );

        place(&mut executor, 1, 0, &[imp]);
        place(&mut executor, 2, 16, &[imp]);

        for _ in 0..5 {
            assert!(executor.step());
        }

        assert_eq!(executor.living_warrior_ids(), vec![1, 2]);
        assert_eq!(executor.process_count(1), Some(1));
        assert_eq!(executor.process_count(2), Some(1));
    }

    #[test]
    fn imp_emits_execute_and_memory_write_each_cycle() {
        let mut executor = Executor::new(config());
        let imp = Instruction::new(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            1,
        );

        place(&mut executor, 1, 0, &[imp]);
        place(
            &mut executor,
            2,
            16,
            &[Instruction::new(
                Opcode::NOP,
                Modifier::F,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                0,
            )],
        );

        assert!(executor.step());

        let events = executor.event_bus.drain();
        let warrior_one_events: Vec<_> = events
            .iter()
            .filter(|event| event.event.warrior_id() == Some(1))
            .collect();
        assert_eq!(warrior_one_events.len(), 2);
        assert!(warrior_one_events.iter().any(|event| {
            event
                == &&TimedEvent {
                    cycle: 0,
                    event: CoreEvent::Execute {
                        address: 0,
                        warrior_id: 1,
                    },
                }
        }));
        assert!(warrior_one_events.iter().any(|event| {
            matches!(
                event,
                &&TimedEvent {
                    cycle: 0,
                    event: CoreEvent::MemoryWrite {
                        address: 1,
                        warrior_id: 1,
                        instruction,
                    },
                } if *instruction == imp
            )
        }));
    }

    #[test]
    fn dat_kills_the_current_process() {
        let mut executor = Executor::new(config());
        place(
            &mut executor,
            1,
            0,
            &[Instruction::new(
                Opcode::DAT,
                Modifier::F,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                0,
            )],
        );
        place(
            &mut executor,
            2,
            8,
            &[Instruction::new(
                Opcode::NOP,
                Modifier::F,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                0,
            )],
        );

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
    fn spl_creates_a_new_process() {
        let mut executor = Executor::new(config());
        place(
            &mut executor,
            1,
            0,
            &[
                Instruction::new(
                    Opcode::SPL,
                    Modifier::B,
                    AddressingMode::Direct,
                    1,
                    AddressingMode::Direct,
                    0,
                ),
                Instruction::new(
                    Opcode::NOP,
                    Modifier::F,
                    AddressingMode::Direct,
                    0,
                    AddressingMode::Direct,
                    0,
                ),
            ],
        );
        place(
            &mut executor,
            2,
            8,
            &[Instruction::new(
                Opcode::NOP,
                Modifier::F,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                0,
            )],
        );

        assert!(executor.step());

        assert_eq!(executor.process_count(1), Some(2));
        assert!(executor.event_bus.drain().contains(&TimedEvent {
            cycle: 0,
            event: CoreEvent::ProcessCreated {
                warrior_id: 1,
                address: 1,
            },
        }));
    }

    #[test]
    fn add_updates_the_destination_field() {
        let mut executor = Executor::new(config());
        place(
            &mut executor,
            1,
            0,
            &[
                Instruction::new(
                    Opcode::ADD,
                    Modifier::AB,
                    AddressingMode::Immediate,
                    1,
                    AddressingMode::Direct,
                    1,
                ),
                Instruction::new(
                    Opcode::DAT,
                    Modifier::F,
                    AddressingMode::Direct,
                    0,
                    AddressingMode::Direct,
                    0,
                ),
            ],
        );
        place(
            &mut executor,
            2,
            8,
            &[Instruction::new(
                Opcode::NOP,
                Modifier::F,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                0,
            )],
        );

        assert!(executor.step());

        assert_eq!(executor.core.read(1).b_value, 1);
    }
}
