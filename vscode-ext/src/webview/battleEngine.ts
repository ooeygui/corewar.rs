export type Opcode =
  | 'DAT'
  | 'MOV'
  | 'ADD'
  | 'SUB'
  | 'MUL'
  | 'DIV'
  | 'MOD'
  | 'JMP'
  | 'JMZ'
  | 'JMN'
  | 'DJN'
  | 'SEQ'
  | 'SNE'
  | 'SLT'
  | 'SPL'
  | 'NOP';

export type Modifier = 'A' | 'B' | 'AB' | 'BA' | 'F' | 'X' | 'I';
export type AddressingMode = '#' | '$' | '*' | '@' | '<' | '>' | '{' | '}';

export interface Operand {
  mode: AddressingMode;
  value: number;
}

export interface Instruction {
  opcode: Opcode;
  modifier: Modifier;
  a: Operand;
  b: Operand;
}

export interface CoreCell {
  instruction: Instruction;
  ownerId: number | null;
}

export interface WarriorSnapshot {
  id: number;
  name: string;
  color: string;
  loadAddress: number;
  processCount: number;
  processes: number[];
  alive: boolean;
}

export interface BattleSummary {
  cycle: number;
  complete: boolean;
  winnerId: number | null;
  warriors: WarriorSnapshot[];
}

export interface BattleState extends BattleSummary {
  core: CoreCell[];
}

export interface CellUpdate {
  address: number;
  cell: CoreCell;
}

export interface BattleCompleteResult {
  cycle: number;
  winnerId: number | null;
  survivors: number[];
}

interface WarriorDefinition {
  id: number;
  name: string;
  color: string;
  source: string;
  program: Instruction[];
  startOffset: number;
}

interface WarriorRuntime extends WarriorDefinition {
  loadAddress: number;
  processes: number[];
  alive: boolean;
}

interface ResolvedOperand {
  immediate: boolean;
  value: number;
  address: number | null;
  instruction: Instruction;
}

interface BattleEventMap {
  reset: BattleState;
  execute: { address: number; warriorId: number; cycle: number };
  memoryWrite: { warriorId: number; updates: CellUpdate[] };
  processChange: WarriorSnapshot;
  battleComplete: BattleCompleteResult;
  warriorLoaded: WarriorSnapshot;
}

type Listener<K extends keyof BattleEventMap> = (payload: BattleEventMap[K]) => void;

const OPCODES = new Set<Opcode>([
  'DAT',
  'MOV',
  'ADD',
  'SUB',
  'MUL',
  'DIV',
  'MOD',
  'JMP',
  'JMZ',
  'JMN',
  'DJN',
  'SEQ',
  'SNE',
  'SLT',
  'SPL',
  'NOP'
]);
const MODIFIERS = new Set<Modifier>(['A', 'B', 'AB', 'BA', 'F', 'X', 'I']);
const RESERVED_WORDS = new Set<string>([
  ...Array.from(OPCODES),
  'ORG',
  'END',
  'EQU',
  'PIN',
  'CMP'
]);

export class BattleEngine {
  private readonly listeners = new Map<keyof BattleEventMap, Set<Listener<keyof BattleEventMap>>>();
  private readonly coreSize: number;
  private readonly maxCycles: number;
  private readonly maxProcesses: number;
  private core: CoreCell[];
  private warriors: WarriorDefinition[] = [];
  private runtimes: WarriorRuntime[] = [];
  private nextWarriorId = 1;
  private cycle = 0;
  private complete = false;
  private winnerId: number | null = null;
  private currentWarriorIndex = 0;

  public constructor(coreSize = 8000, maxCycles = coreSize * 10, maxProcesses = coreSize) {
    this.coreSize = coreSize;
    this.maxCycles = maxCycles;
    this.maxProcesses = maxProcesses;
    this.core = this.createEmptyCore();
  }

  public on<K extends keyof BattleEventMap>(event: K, listener: Listener<K>): () => void {
    const listeners = this.listeners.get(event) ?? new Set<Listener<keyof BattleEventMap>>();
    listeners.add(listener as Listener<keyof BattleEventMap>);
    this.listeners.set(event, listeners);
    return () => {
      listeners.delete(listener as Listener<keyof BattleEventMap>);
    };
  }

  public loadWarrior(source: string, name: string): WarriorSnapshot {
    const parsed = this.parseWarrior(source);
    const definition: WarriorDefinition = {
      id: this.nextWarriorId,
      name,
      color: this.generateWarriorColor(this.nextWarriorId - 1),
      source,
      program: parsed.program,
      startOffset: parsed.startOffset
    };

    this.nextWarriorId += 1;
    this.warriors.push(definition);
    this.reset();

    const snapshot = this.getWarriorSnapshot(definition.id);
    this.emit('warriorLoaded', snapshot);
    return snapshot;
  }

  public removeWarrior(id: number): void {
    this.warriors = this.warriors.filter((warrior) => warrior.id !== id);
    this.reset();
  }

  public reset(): void {
    this.core = this.createEmptyCore();
    this.cycle = 0;
    this.complete = false;
    this.winnerId = null;
    this.currentWarriorIndex = 0;

    if (this.warriors.length === 0) {
      this.runtimes = [];
      this.emit('reset', this.getState());
      return;
    }

    const spacing = Math.max(1, Math.floor(this.coreSize / this.warriors.length));
    this.runtimes = this.warriors.map((warrior, index) => {
      const loadAddress = this.wrap(index * spacing);
      warrior.program.forEach((instruction, instructionIndex) => {
        const address = this.wrap(loadAddress + instructionIndex);
        this.core[address] = {
          instruction: this.cloneInstruction(instruction),
          ownerId: warrior.id
        };
      });

      return {
        ...warrior,
        loadAddress,
        processes: [this.wrap(loadAddress + warrior.startOffset)],
        alive: true
      };
    });

    this.emit('reset', this.getState());
  }

  public step(): BattleSummary {
    if (this.complete || this.runtimes.length === 0) {
      return this.getSummary();
    }

    const nextIndex = this.findNextWarriorIndex();
    if (nextIndex === -1) {
      this.finishBattle();
      return this.getSummary();
    }

    const warrior = this.runtimes[nextIndex];
    const pc = warrior.processes.shift();
    if (pc === undefined) {
      warrior.alive = false;
      this.emit('processChange', this.snapshotRuntime(warrior));
      this.currentWarriorIndex = (nextIndex + 1) % Math.max(1, this.runtimes.length);
      this.checkForCompletion();
      return this.getSummary();
    }

    this.emit('execute', { address: pc, warriorId: warrior.id, cycle: this.cycle });
    this.executeInstruction(warrior, pc);
    warrior.alive = warrior.processes.length > 0;
    this.emit('processChange', this.snapshotRuntime(warrior));

    this.cycle += 1;
    this.currentWarriorIndex = (nextIndex + 1) % Math.max(1, this.runtimes.length);
    this.checkForCompletion();
    return this.getSummary();
  }

  public run(cycles: number): BattleSummary {
    let remaining = Math.max(0, Math.floor(cycles));
    while (remaining > 0 && !this.complete) {
      this.step();
      remaining -= 1;
    }
    return this.getSummary();
  }

  public getState(): BattleState {
    return {
      ...this.getSummary(),
      core: this.core.map((cell) => ({
        instruction: this.cloneInstruction(cell.instruction),
        ownerId: cell.ownerId
      }))
    };
  }

  public getSummary(): BattleSummary {
    return {
      cycle: this.cycle,
      complete: this.complete,
      winnerId: this.winnerId,
      warriors: this.runtimes.map((warrior) => this.snapshotRuntime(warrior))
    };
  }

  private emit<K extends keyof BattleEventMap>(event: K, payload: BattleEventMap[K]): void {
    const listeners = this.listeners.get(event);
    if (listeners === undefined) {
      return;
    }

    listeners.forEach((listener) => {
      (listener as Listener<K>)(payload);
    });
  }

  private createEmptyCore(): CoreCell[] {
    return Array.from({ length: this.coreSize }, () => ({
      instruction: this.makeDatInstruction(),
      ownerId: null
    }));
  }

  private makeDatInstruction(): Instruction {
    return {
      opcode: 'DAT',
      modifier: 'F',
      a: { mode: '$', value: 0 },
      b: { mode: '$', value: 0 }
    };
  }

  private findNextWarriorIndex(): number {
    for (let offset = 0; offset < this.runtimes.length; offset += 1) {
      const index = (this.currentWarriorIndex + offset) % this.runtimes.length;
      const warrior = this.runtimes[index];
      if (warrior.alive && warrior.processes.length > 0) {
        return index;
      }
    }

    return -1;
  }

  private executeInstruction(warrior: WarriorRuntime, pc: number): void {
    const instruction = this.cloneInstruction(this.core[pc].instruction);
    const writtenAddresses = new Map<number, CellUpdate>();
    const markWrite = (address: number): void => {
      writtenAddresses.set(address, {
        address,
        cell: {
          instruction: this.cloneInstruction(this.core[address].instruction),
          ownerId: this.core[address].ownerId
        }
      });
    };

    const source = this.resolveOperand(pc, instruction.a, warrior.id, markWrite);
    const destination = this.resolveOperand(pc, instruction.b, warrior.id, markWrite);
    const nextPc = this.wrap(pc + 1);

    switch (instruction.opcode) {
      case 'DAT':
        break;
      case 'MOV':
        if (!destination.immediate && destination.address !== null) {
          this.applyMovModifier(instruction.modifier, source, destination.address);
          this.core[destination.address].ownerId = warrior.id;
          markWrite(destination.address);
          warrior.processes.push(nextPc);
        }
        break;
      case 'ADD':
      case 'SUB':
      case 'MUL':
      case 'DIV':
      case 'MOD':
        if (destination.immediate || destination.address === null) {
          warrior.processes.push(nextPc);
          break;
        }
        if (!this.applyArithmeticModifier(instruction.opcode, instruction.modifier, source, destination.address)) {
          break;
        }
        this.core[destination.address].ownerId = warrior.id;
        markWrite(destination.address);
        warrior.processes.push(nextPc);
        break;
      case 'JMP':
        warrior.processes.push(source.address ?? this.wrap(pc + source.value));
        break;
      case 'JMZ':
        warrior.processes.push(this.shouldJumpIfZero(instruction.modifier, destination) ? (source.address ?? this.wrap(pc + source.value)) : nextPc);
        break;
      case 'JMN':
        warrior.processes.push(this.shouldJumpIfNonZero(instruction.modifier, destination) ? (source.address ?? this.wrap(pc + source.value)) : nextPc);
        break;
      case 'DJN': {
        if (destination.immediate || destination.address === null) {
          warrior.processes.push(nextPc);
          break;
        }
        const stillNonZero = this.decrementAndTest(instruction.modifier, destination.address, warrior.id);
        markWrite(destination.address);
        warrior.processes.push(stillNonZero ? (source.address ?? this.wrap(pc + source.value)) : nextPc);
        break;
      }
      case 'SEQ':
        warrior.processes.push(this.instructionsMatch(instruction.modifier, source, destination) ? this.wrap(pc + 2) : nextPc);
        break;
      case 'SNE':
        warrior.processes.push(this.instructionsMatch(instruction.modifier, source, destination) ? nextPc : this.wrap(pc + 2));
        break;
      case 'SLT':
        warrior.processes.push(this.isSourceLessThanDestination(instruction.modifier, source, destination) ? this.wrap(pc + 2) : nextPc);
        break;
      case 'SPL': {
        const target = source.address ?? this.wrap(pc + source.value);
        warrior.processes.push(nextPc);
        if (warrior.processes.length < this.maxProcesses) {
          warrior.processes.push(target);
        }
        break;
      }
      case 'NOP':
        warrior.processes.push(nextPc);
        break;
      default:
        warrior.processes.push(nextPc);
        break;
    }

    if (writtenAddresses.size > 0) {
      this.emit('memoryWrite', {
        warriorId: warrior.id,
        updates: Array.from(writtenAddresses.values())
      });
    }
  }

  private applyMovModifier(modifier: Modifier, source: ResolvedOperand, destinationAddress: number): void {
    const destination = this.core[destinationAddress].instruction;

    switch (modifier) {
      case 'A':
        destination.a = this.cloneOperand(source.instruction.a);
        break;
      case 'B':
        destination.b = this.cloneOperand(source.instruction.b);
        break;
      case 'AB':
        destination.b = this.cloneOperand(source.instruction.a);
        break;
      case 'BA':
        destination.a = this.cloneOperand(source.instruction.b);
        break;
      case 'F':
        destination.a = this.cloneOperand(source.instruction.a);
        destination.b = this.cloneOperand(source.instruction.b);
        break;
      case 'X':
        destination.a = this.cloneOperand(source.instruction.b);
        destination.b = this.cloneOperand(source.instruction.a);
        break;
      case 'I':
        this.core[destinationAddress].instruction = this.cloneInstruction(source.instruction);
        break;
    }
  }

  private applyArithmeticModifier(
    opcode: Extract<Opcode, 'ADD' | 'SUB' | 'MUL' | 'DIV' | 'MOD'>,
    modifier: Modifier,
    source: ResolvedOperand,
    destinationAddress: number
  ): boolean {
    const destination = this.core[destinationAddress].instruction;
    const operations = this.getModifierOperationPairs(modifier, source.instruction, destination);

    for (const operation of operations) {
      if ((opcode === 'DIV' || opcode === 'MOD') && operation.source === 0) {
        return false;
      }
    }

    operations.forEach((operation) => {
      const updatedValue = this.applyArithmetic(opcode, operation.source, operation.destination);
      if (operation.field === 'a') {
        destination.a.value = updatedValue;
      } else {
        destination.b.value = updatedValue;
      }
    });

    return true;
  }

  private getModifierOperationPairs(
    modifier: Modifier,
    source: Instruction,
    destination: Instruction
  ): Array<{ field: 'a' | 'b'; source: number; destination: number }> {
    switch (modifier) {
      case 'A':
        return [{ field: 'a', source: source.a.value, destination: destination.a.value }];
      case 'B':
        return [{ field: 'b', source: source.b.value, destination: destination.b.value }];
      case 'AB':
        return [{ field: 'b', source: source.a.value, destination: destination.b.value }];
      case 'BA':
        return [{ field: 'a', source: source.b.value, destination: destination.a.value }];
      case 'F':
      case 'I':
        return [
          { field: 'a', source: source.a.value, destination: destination.a.value },
          { field: 'b', source: source.b.value, destination: destination.b.value }
        ];
      case 'X':
        return [
          { field: 'a', source: source.b.value, destination: destination.a.value },
          { field: 'b', source: source.a.value, destination: destination.b.value }
        ];
    }
  }

  private applyArithmetic(opcode: Extract<Opcode, 'ADD' | 'SUB' | 'MUL' | 'DIV' | 'MOD'>, source: number, destination: number): number {
    switch (opcode) {
      case 'ADD':
        return this.normalizeField(destination + source);
      case 'SUB':
        return this.normalizeField(destination - source);
      case 'MUL':
        return this.normalizeField(destination * source);
      case 'DIV':
        return this.normalizeField(Math.trunc(destination / source));
      case 'MOD':
        return this.normalizeField(destination % source);
    }
  }

  private shouldJumpIfZero(modifier: Modifier, destination: ResolvedOperand): boolean {
    switch (modifier) {
      case 'A':
      case 'BA':
        return destination.instruction.a.value === 0;
      case 'B':
      case 'AB':
        return destination.instruction.b.value === 0;
      case 'F':
      case 'X':
      case 'I':
        return destination.instruction.a.value === 0 && destination.instruction.b.value === 0;
    }
  }

  private shouldJumpIfNonZero(modifier: Modifier, destination: ResolvedOperand): boolean {
    switch (modifier) {
      case 'A':
      case 'BA':
        return destination.instruction.a.value !== 0;
      case 'B':
      case 'AB':
        return destination.instruction.b.value !== 0;
      case 'F':
      case 'X':
      case 'I':
        return destination.instruction.a.value !== 0 || destination.instruction.b.value !== 0;
    }
  }

  private decrementAndTest(modifier: Modifier, destinationAddress: number, warriorId: number): boolean {
    const destination = this.core[destinationAddress].instruction;
    this.core[destinationAddress].ownerId = warriorId;

    switch (modifier) {
      case 'A':
      case 'BA':
        destination.a.value = this.normalizeField(destination.a.value - 1);
        return destination.a.value !== 0;
      case 'B':
      case 'AB':
        destination.b.value = this.normalizeField(destination.b.value - 1);
        return destination.b.value !== 0;
      case 'F':
      case 'X':
      case 'I':
        destination.a.value = this.normalizeField(destination.a.value - 1);
        destination.b.value = this.normalizeField(destination.b.value - 1);
        return destination.a.value !== 0 || destination.b.value !== 0;
    }
  }

  private instructionsMatch(modifier: Modifier, source: ResolvedOperand, destination: ResolvedOperand): boolean {
    switch (modifier) {
      case 'A':
        return source.instruction.a.value === destination.instruction.a.value;
      case 'B':
        return source.instruction.b.value === destination.instruction.b.value;
      case 'AB':
        return source.instruction.a.value === destination.instruction.b.value;
      case 'BA':
        return source.instruction.b.value === destination.instruction.a.value;
      case 'F':
        return (
          source.instruction.a.value === destination.instruction.a.value &&
          source.instruction.b.value === destination.instruction.b.value
        );
      case 'X':
        return (
          source.instruction.a.value === destination.instruction.b.value &&
          source.instruction.b.value === destination.instruction.a.value
        );
      case 'I':
        return this.instructionsEqual(source.instruction, destination.instruction);
    }
  }

  private isSourceLessThanDestination(modifier: Modifier, source: ResolvedOperand, destination: ResolvedOperand): boolean {
    switch (modifier) {
      case 'A':
        return source.instruction.a.value < destination.instruction.a.value;
      case 'B':
        return source.instruction.b.value < destination.instruction.b.value;
      case 'AB':
        return source.instruction.a.value < destination.instruction.b.value;
      case 'BA':
        return source.instruction.b.value < destination.instruction.a.value;
      case 'F':
        return (
          source.instruction.a.value < destination.instruction.a.value &&
          source.instruction.b.value < destination.instruction.b.value
        );
      case 'X':
        return (
          source.instruction.a.value < destination.instruction.b.value &&
          source.instruction.b.value < destination.instruction.a.value
        );
      case 'I':
        return (
          source.instruction.a.value + source.instruction.b.value <
          destination.instruction.a.value + destination.instruction.b.value
        );
    }
  }

  private resolveOperand(
    pc: number,
    operand: Operand,
    warriorId: number,
    markWrite: (address: number) => void
  ): ResolvedOperand {
    if (operand.mode === '#') {
      return {
        immediate: true,
        value: operand.value,
        address: null,
        instruction: {
          opcode: 'DAT',
          modifier: 'F',
          a: this.cloneOperand(operand),
          b: this.cloneOperand(operand)
        }
      };
    }

    const baseAddress = this.wrap(pc + operand.value);
    switch (operand.mode) {
      case '$':
        return this.readResolvedCell(baseAddress);
      case '*': {
        const pointer = this.core[baseAddress].instruction.a.value;
        return this.readResolvedCell(this.wrap(baseAddress + pointer));
      }
      case '@': {
        const pointer = this.core[baseAddress].instruction.b.value;
        return this.readResolvedCell(this.wrap(baseAddress + pointer));
      }
      case '{': {
        const cell = this.core[baseAddress];
        cell.instruction.a.value = this.normalizeField(cell.instruction.a.value - 1);
        cell.ownerId = warriorId;
        markWrite(baseAddress);
        return this.readResolvedCell(this.wrap(baseAddress + cell.instruction.a.value));
      }
      case '<': {
        const cell = this.core[baseAddress];
        cell.instruction.b.value = this.normalizeField(cell.instruction.b.value - 1);
        cell.ownerId = warriorId;
        markWrite(baseAddress);
        return this.readResolvedCell(this.wrap(baseAddress + cell.instruction.b.value));
      }
      case '}': {
        const cell = this.core[baseAddress];
        const resolvedAddress = this.wrap(baseAddress + cell.instruction.a.value);
        cell.instruction.a.value = this.normalizeField(cell.instruction.a.value + 1);
        cell.ownerId = warriorId;
        markWrite(baseAddress);
        return this.readResolvedCell(resolvedAddress);
      }
      case '>': {
        const cell = this.core[baseAddress];
        const resolvedAddress = this.wrap(baseAddress + cell.instruction.b.value);
        cell.instruction.b.value = this.normalizeField(cell.instruction.b.value + 1);
        cell.ownerId = warriorId;
        markWrite(baseAddress);
        return this.readResolvedCell(resolvedAddress);
      }
      default:
        return this.readResolvedCell(baseAddress);
    }
  }

  private readResolvedCell(address: number): ResolvedOperand {
    const cell = this.core[address];
    return {
      immediate: false,
      value: 0,
      address,
      instruction: this.cloneInstruction(cell.instruction)
    };
  }

  private instructionsEqual(left: Instruction, right: Instruction): boolean {
    return (
      left.opcode === right.opcode &&
      left.modifier === right.modifier &&
      left.a.mode === right.a.mode &&
      left.a.value === right.a.value &&
      left.b.mode === right.b.mode &&
      left.b.value === right.b.value
    );
  }

  private checkForCompletion(): void {
    if (this.complete) {
      return;
    }

    const survivors = this.runtimes.filter((warrior) => warrior.alive && warrior.processes.length > 0);
    if (this.cycle >= this.maxCycles) {
      this.finishBattle();
      return;
    }

    if (this.runtimes.length > 1 && survivors.length <= 1) {
      this.finishBattle();
    }
  }

  private finishBattle(): void {
    this.complete = true;
    const survivors = this.runtimes.filter((warrior) => warrior.alive && warrior.processes.length > 0);
    this.winnerId = survivors.length === 1 ? survivors[0].id : null;
    this.emit('battleComplete', {
      cycle: this.cycle,
      winnerId: this.winnerId,
      survivors: survivors.map((warrior) => warrior.id)
    });
  }

  private getWarriorSnapshot(id: number): WarriorSnapshot {
    const runtime = this.runtimes.find((warrior) => warrior.id === id);
    if (runtime !== undefined) {
      return this.snapshotRuntime(runtime);
    }

    const definition = this.warriors.find((warrior) => warrior.id === id);
    if (definition === undefined) {
      throw new Error(`Unknown warrior ${id}`);
    }

    return {
      id: definition.id,
      name: definition.name,
      color: definition.color,
      loadAddress: 0,
      processCount: 0,
      processes: [],
      alive: false
    };
  }

  private snapshotRuntime(runtime: WarriorRuntime): WarriorSnapshot {
    return {
      id: runtime.id,
      name: runtime.name,
      color: runtime.color,
      loadAddress: runtime.loadAddress,
      processCount: runtime.processes.length,
      processes: [...runtime.processes],
      alive: runtime.alive
    };
  }

  private wrap(value: number): number {
    return ((Math.trunc(value) % this.coreSize) + this.coreSize) % this.coreSize;
  }

  private normalizeField(value: number): number {
    const wrapped = this.wrap(value);
    const midpoint = Math.floor(this.coreSize / 2);
    return wrapped > midpoint ? wrapped - this.coreSize : wrapped;
  }

  private cloneInstruction(instruction: Instruction): Instruction {
    return {
      opcode: instruction.opcode,
      modifier: instruction.modifier,
      a: this.cloneOperand(instruction.a),
      b: this.cloneOperand(instruction.b)
    };
  }

  private cloneOperand(operand: Operand): Operand {
    return {
      mode: operand.mode,
      value: operand.value
    };
  }

  private generateWarriorColor(index: number): string {
    const hue = (index * 137.508) % 360;
    return `hsl(${hue.toFixed(1)}deg 72% 62%)`;
  }

  private parseWarrior(source: string): { program: Instruction[]; startOffset: number } {
    const constants = new Map<string, string>();
    const labels = new Map<string, number>();
    const rawInstructions: string[] = [];
    const pendingLabels: string[] = [];
    let orgExpression: string | undefined;
    let endExpression: string | undefined;

    const lines = source.split(/\r?\n/);
    for (const rawLine of lines) {
      const line = this.stripComment(rawLine).trim();
      if (!line) {
        continue;
      }

      const { label, statement } = this.extractLabel(line);
      if (!statement) {
        if (label !== undefined) {
          pendingLabels.push(label);
        }
        continue;
      }

      const tokens = statement.split(/\s+/);
      const keyword = tokens[0].toUpperCase();
      const remainder = statement.slice(tokens[0].length).trim();
      const allLabels = [...pendingLabels];
      pendingLabels.length = 0;
      if (label !== undefined) {
        allLabels.push(label);
      }

      if (keyword === 'EQU') {
        const constantLabel = allLabels[0];
        if (constantLabel !== undefined) {
          constants.set(constantLabel, remainder);
        }
        continue;
      }

      if (keyword === 'ORG') {
        orgExpression = remainder || '0';
        continue;
      }

      if (keyword === 'END') {
        endExpression = remainder || orgExpression || '0';
        continue;
      }

      if (keyword === 'PIN') {
        continue;
      }

      allLabels.forEach((pendingLabel) => {
        labels.set(pendingLabel, rawInstructions.length);
      });
      rawInstructions.push(statement);
    }

    const resolveExpression = (expression: string, resolving = new Set<string>()): number => {
      const trimmed = expression.trim();
      if (!trimmed) {
        return 0;
      }

      const tokens = trimmed.match(/[A-Za-z_][A-Za-z0-9_$]*|[-+*/()%]|\d+/g) ?? [];
      let index = 0;

      const parsePrimary = (): number => {
        const token = tokens[index];
        if (token === undefined) {
          return 0;
        }
        if (token === '(') {
          index += 1;
          const value = parseExpression();
          if (tokens[index] === ')') {
            index += 1;
          }
          return value;
        }
        if (token === '+' || token === '-') {
          index += 1;
          const value = parsePrimary();
          return token === '-' ? -value : value;
        }
        index += 1;
        if (/^\d+$/.test(token)) {
          return Number.parseInt(token, 10);
        }

        const symbol = token.toUpperCase();
        if (resolving.has(symbol)) {
          return 0;
        }
        if (labels.has(symbol)) {
          return labels.get(symbol) ?? 0;
        }
        if (constants.has(symbol)) {
          resolving.add(symbol);
          const value = resolveExpression(constants.get(symbol) ?? '0', resolving);
          resolving.delete(symbol);
          return value;
        }
        if (symbol === 'CORESIZE') {
          return this.coreSize;
        }
        return 0;
      };

      const parseFactor = (): number => {
        let value = parsePrimary();
        while (tokens[index] === '*' || tokens[index] === '/' || tokens[index] === '%') {
          const operator = tokens[index];
          index += 1;
          const right = parsePrimary();
          if (operator === '*') {
            value *= right;
          } else if (operator === '/') {
            value = right === 0 ? value : Math.trunc(value / right);
          } else {
            value = right === 0 ? value : value % right;
          }
        }
        return value;
      };

      const parseExpression = (): number => {
        let value = parseFactor();
        while (tokens[index] === '+' || tokens[index] === '-') {
          const operator = tokens[index];
          index += 1;
          const right = parseFactor();
          value = operator === '+' ? value + right : value - right;
        }
        return value;
      };

      return parseExpression();
    };

    const parseOperand = (value: string | undefined): Operand => {
      const trimmed = value?.trim() ?? '';
      if (!trimmed) {
        return { mode: '$', value: 0 };
      }
      const firstCharacter = trimmed[0] as AddressingMode;
      const mode: AddressingMode = ['#', '$', '*', '@', '<', '>', '{', '}'].includes(firstCharacter)
        ? firstCharacter
        : '$';
      const expression = mode === '$' ? trimmed : trimmed.slice(1).trim();
      return {
        mode,
        value: this.normalizeField(resolveExpression(expression || '0'))
      };
    };

    const program = rawInstructions.map((statement) => {
      const match = statement.match(/^([A-Za-z]{3})(?:\.(A|B|AB|BA|F|X|I))?(?:\s+(.*))?$/i);
      if (match === null) {
        throw new Error(`Unsupported instruction: ${statement}`);
      }

      const opcodeText = match[1].toUpperCase();
      const opcode = (opcodeText === 'CMP' ? 'SEQ' : opcodeText) as Opcode;
      if (!OPCODES.has(opcode)) {
        throw new Error(`Unsupported opcode: ${opcode}`);
      }

      const operands = (match[3] ?? '').split(',');
      const a = parseOperand(operands[0]);
      const b = parseOperand(operands[1]);
      const modifierText = match[2]?.toUpperCase() as Modifier | undefined;
      const modifier = modifierText !== undefined && MODIFIERS.has(modifierText)
        ? modifierText
        : this.inferModifier(opcode, a.mode, b.mode);

      return {
        opcode,
        modifier,
        a,
        b
      };
    });

    const startOffset = this.wrap(
      this.normalizeField(resolveExpression(endExpression ?? orgExpression ?? '0'))
    );
    return { program, startOffset };
  }

  private inferModifier(opcode: Opcode, aMode: AddressingMode, bMode: AddressingMode): Modifier {
    if (opcode === 'DAT' || opcode === 'NOP') {
      return 'F';
    }
    if (opcode === 'JMP' || opcode === 'JMZ' || opcode === 'JMN' || opcode === 'DJN' || opcode === 'SPL') {
      return 'B';
    }
    if (opcode === 'SLT') {
      return aMode === '#' && bMode !== '#' ? 'AB' : 'B';
    }
    if (opcode === 'MOV' || opcode === 'SEQ' || opcode === 'SNE') {
      if (aMode === '#' && bMode !== '#') {
        return 'AB';
      }
      if (aMode !== '#' && bMode === '#') {
        return 'B';
      }
      return 'I';
    }
    if (aMode === '#' && bMode !== '#') {
      return 'AB';
    }
    if (aMode !== '#' && bMode === '#') {
      return 'B';
    }
    return 'F';
  }

  private stripComment(line: string): string {
    const commentIndex = line.indexOf(';');
    return commentIndex >= 0 ? line.slice(0, commentIndex) : line;
  }

  private extractLabel(line: string): { label: string | undefined; statement: string } {
    const colonMatch = line.match(/^([A-Za-z_][A-Za-z0-9_$]*):\s*(.*)$/);
    if (colonMatch !== null) {
      return {
        label: colonMatch[1].toUpperCase(),
        statement: colonMatch[2] ?? ''
      };
    }

    const parts = line.split(/\s+/);
    if (parts.length > 1) {
      const candidate = parts[0].toUpperCase();
      if (!RESERVED_WORDS.has(candidate) && /^[A-Za-z_][A-Za-z0-9_$]*$/.test(parts[0])) {
        return {
          label: candidate,
          statement: line.slice(parts[0].length).trimStart()
        };
      }
    }

    return { label: undefined, statement: line };
  }
}
