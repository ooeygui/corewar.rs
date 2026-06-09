use corewar_core::{AddressingMode, Core, CoreEvent, EventBus, Instruction};

pub fn resolve_a(core: &mut Core, pc: usize, instruction: &Instruction) -> usize {
    resolve_operand(core, pc, instruction.a_mode, instruction.a_value, None)
}

pub fn resolve_b(core: &mut Core, pc: usize, instruction: &Instruction) -> usize {
    resolve_operand(core, pc, instruction.b_mode, instruction.b_value, None)
}

pub(crate) fn resolve_a_with_events(
    core: &mut Core,
    pc: usize,
    instruction: &Instruction,
    warrior_id: u32,
    event_bus: &EventBus,
) -> usize {
    resolve_operand(
        core,
        pc,
        instruction.a_mode,
        instruction.a_value,
        Some((warrior_id, event_bus)),
    )
}

pub(crate) fn resolve_b_with_events(
    core: &mut Core,
    pc: usize,
    instruction: &Instruction,
    warrior_id: u32,
    event_bus: &EventBus,
) -> usize {
    resolve_operand(
        core,
        pc,
        instruction.b_mode,
        instruction.b_value,
        Some((warrior_id, event_bus)),
    )
}

fn resolve_operand(
    core: &mut Core,
    pc: usize,
    mode: AddressingMode,
    value: i32,
    event_target: Option<(u32, &EventBus)>,
) -> usize {
    let base = core.normalize(pc as i32 + value);

    match mode {
        AddressingMode::Immediate | AddressingMode::Direct => base,
        AddressingMode::IndirectA => indirect_a(core, base, event_target),
        AddressingMode::IndirectB => indirect_b(core, base, event_target),
        AddressingMode::PreDecIndirectB => pre_decrement_b(core, base, event_target),
        AddressingMode::PostIncIndirectB => post_increment_b(core, base, event_target),
        AddressingMode::PreDecIndirectA => pre_decrement_a(core, base, event_target),
        AddressingMode::PostIncIndirectA => post_increment_a(core, base, event_target),
    }
}

fn indirect_a(core: &Core, base: usize, event_target: Option<(u32, &EventBus)>) -> usize {
    emit_read(base, event_target);
    let offset = core.read(base).a_value;
    core.normalize(base as i32 + offset)
}

fn indirect_b(core: &Core, base: usize, event_target: Option<(u32, &EventBus)>) -> usize {
    emit_read(base, event_target);
    let offset = core.read(base).b_value;
    core.normalize(base as i32 + offset)
}

fn pre_decrement_a(core: &mut Core, base: usize, event_target: Option<(u32, &EventBus)>) -> usize {
    emit_read(base, event_target);
    let size = core.size() as i64;
    let decremented = {
        let cell = core.read_mut(base);
        cell.a_value = normalize_field(size, i64::from(cell.a_value) - 1);
        cell.a_value
    };
    emit_write(core, base, event_target);
    core.normalize(base as i32 + decremented)
}

fn pre_decrement_b(core: &mut Core, base: usize, event_target: Option<(u32, &EventBus)>) -> usize {
    emit_read(base, event_target);
    let size = core.size() as i64;
    let decremented = {
        let cell = core.read_mut(base);
        cell.b_value = normalize_field(size, i64::from(cell.b_value) - 1);
        cell.b_value
    };
    emit_write(core, base, event_target);
    core.normalize(base as i32 + decremented)
}

fn post_increment_a(core: &mut Core, base: usize, event_target: Option<(u32, &EventBus)>) -> usize {
    let target = indirect_a(core, base, event_target);
    let size = core.size() as i64;
    {
        let cell = core.read_mut(base);
        cell.a_value = normalize_field(size, i64::from(cell.a_value) + 1);
    }
    emit_write(core, base, event_target);
    target
}

fn post_increment_b(core: &mut Core, base: usize, event_target: Option<(u32, &EventBus)>) -> usize {
    let target = indirect_b(core, base, event_target);
    let size = core.size() as i64;
    {
        let cell = core.read_mut(base);
        cell.b_value = normalize_field(size, i64::from(cell.b_value) + 1);
    }
    emit_write(core, base, event_target);
    target
}

fn normalize_field(size: i64, value: i64) -> i32 {
    value.rem_euclid(size) as i32
}

fn emit_read(address: usize, event_target: Option<(u32, &EventBus)>) {
    if let Some((warrior_id, event_bus)) = event_target {
        event_bus.emit(CoreEvent::Read {
            address,
            warrior_id,
        });
    }
}

fn emit_write(core: &Core, address: usize, event_target: Option<(u32, &EventBus)>) {
    if let Some((warrior_id, event_bus)) = event_target {
        event_bus.emit(CoreEvent::MemoryWrite {
            address,
            warrior_id,
            instruction: *core.read(address),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_a, resolve_b};
    use corewar_core::{AddressingMode, Core, Instruction, Modifier, Opcode};

    #[test]
    fn indirect_b_resolves_through_target_field() {
        let mut core = Core::new(10);
        core.load(
            0,
            Instruction::new(
                Opcode::MOV,
                Modifier::I,
                AddressingMode::IndirectB,
                1,
                AddressingMode::Direct,
                0,
            ),
        );
        core.load(
            1,
            Instruction::new(
                Opcode::DAT,
                Modifier::F,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                2,
            ),
        );

        let instruction = *core.read(0);
        assert_eq!(resolve_a(&mut core, 0, &instruction), 3);
    }

    #[test]
    fn predecrement_and_postincrement_apply_side_effects() {
        let mut core = Core::new(10);
        core.load(
            0,
            Instruction::new(
                Opcode::MOV,
                Modifier::I,
                AddressingMode::PreDecIndirectA,
                1,
                AddressingMode::PostIncIndirectB,
                2,
            ),
        );
        core.load(
            1,
            Instruction::new(
                Opcode::DAT,
                Modifier::F,
                AddressingMode::Direct,
                4,
                AddressingMode::Direct,
                0,
            ),
        );
        core.load(
            2,
            Instruction::new(
                Opcode::DAT,
                Modifier::F,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                3,
            ),
        );

        let instruction = *core.read(0);
        assert_eq!(resolve_a(&mut core, 0, &instruction), 4);
        assert_eq!(core.read(1).a_value, 3);
        assert_eq!(resolve_b(&mut core, 0, &instruction), 5);
        assert_eq!(core.read(2).b_value, 4);
    }
}
