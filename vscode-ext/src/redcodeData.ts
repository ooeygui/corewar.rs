export interface OpcodeInfo {
  name: string;
  description: string;
  validModifiers: readonly string[];
  defaultModifier: string;
}

export interface ModifierInfo {
  name: string;
  description: string;
}

export interface DirectiveInfo {
  name: string;
  description: string;
  snippet: string;
}

export interface PredefinedConstantInfo {
  name: string;
  value: number;
  description: string;
}

export interface AddressingModeInfo {
  symbol: string;
  name: string;
  description: string;
}

export const OPCODES: ReadonlyArray<OpcodeInfo> = [
  {
    name: 'DAT',
    description: 'Stores data and terminates the current process when executed.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'F'
  },
  {
    name: 'MOV',
    description: 'Copies the source field or instruction into the destination.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'I'
  },
  {
    name: 'ADD',
    description: 'Adds the source value or fields into the destination.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'F'
  },
  {
    name: 'SUB',
    description: 'Subtracts the source value or fields from the destination.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'F'
  },
  {
    name: 'MUL',
    description: 'Multiplies the destination by the source value or fields.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'F'
  },
  {
    name: 'DIV',
    description: 'Divides the destination by the source value or fields.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'F'
  },
  {
    name: 'MOD',
    description: 'Stores the remainder after dividing the destination by the source.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'F'
  },
  {
    name: 'JMP',
    description: 'Jumps unconditionally to the address resolved from the A-field.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'B'
  },
  {
    name: 'JMZ',
    description: 'Jumps if the tested destination field is zero.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'B'
  },
  {
    name: 'JMN',
    description: 'Jumps if the tested destination field is non-zero.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'B'
  },
  {
    name: 'DJN',
    description: 'Decrements the tested destination field, then jumps if it remains non-zero.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'B'
  },
  {
    name: 'SEQ',
    description: 'Skips the next instruction when the compared values are equal.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'I'
  },
  {
    name: 'SNE',
    description: 'Skips the next instruction when the compared values are not equal.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'I'
  },
  {
    name: 'SLT',
    description: 'Skips the next instruction when the source is less than the destination.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'B'
  },
  {
    name: 'SPL',
    description: 'Splits execution by spawning a new process at the A-field target.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'B'
  },
  {
    name: 'NOP',
    description: 'Performs no operation and advances to the next instruction.',
    validModifiers: ['A', 'B', 'AB', 'BA', 'F', 'X', 'I'],
    defaultModifier: 'F'
  }
];

export const MODIFIERS: ReadonlyArray<ModifierInfo> = [
  { name: 'A', description: 'Use the A-field of the source and the A-field of the destination.' },
  { name: 'B', description: 'Use the B-field of the source and the B-field of the destination.' },
  { name: 'AB', description: 'Use the A-field of the source and the B-field of the destination.' },
  { name: 'BA', description: 'Use the B-field of the source and the A-field of the destination.' },
  { name: 'F', description: 'Use both fields in parallel: A-to-A and B-to-B.' },
  { name: 'X', description: 'Use both fields crosswise: A-to-B and B-to-A.' },
  { name: 'I', description: 'Operate on the entire instruction, including opcode, modifier, and both fields.' }
];

export const DIRECTIVES: ReadonlyArray<DirectiveInfo> = [
  { name: 'ORG', description: 'Sets the initial execution point for the warrior.', snippet: 'ORG ${1:start}' },
  { name: 'END', description: 'Marks the end of the source and can override the start label.', snippet: 'END ${1:start}' },
  { name: 'EQU', description: 'Assigns a symbolic constant to an expression.', snippet: 'EQU ${1:expression}' },
  { name: 'PIN', description: 'Declares the warrior\'s preferred p-space pin number.', snippet: 'PIN ${1:0}' }
];

export const PREDEFINED_CONSTANTS: ReadonlyArray<PredefinedConstantInfo> = [
  { name: 'CORESIZE', value: 8000, description: 'Default size of the core in instructions.' },
  { name: 'MAXPROCESSES', value: 8000, description: 'Default maximum number of processes per warrior.' },
  { name: 'MAXCYCLES', value: 80000, description: 'Default cycle limit before a tie is declared.' },
  { name: 'MAXLENGTH', value: 100, description: 'Default maximum warrior length.' },
  { name: 'MINDISTANCE', value: 100, description: 'Default minimum load distance between warriors.' },
  { name: 'WARRIORS', value: 2, description: 'Default number of warriors in a standard match.' }
];

export const ADDRESSING_MODES: ReadonlyArray<AddressingModeInfo> = [
  { symbol: '#', name: 'Immediate', description: 'Use the literal value in the operand instead of reading core memory.' },
  { symbol: '$', name: 'Direct', description: 'Address the instruction at the given relative offset.' },
  { symbol: '*', name: 'A-indirect', description: 'Follow the A-field of the addressed instruction to find the final target.' },
  { symbol: '@', name: 'B-indirect', description: 'Follow the B-field of the addressed instruction to find the final target.' },
  { symbol: '<', name: 'Predecrement B-indirect', description: 'Decrement the addressed instruction\'s B-field, then use it as an indirect pointer.' },
  { symbol: '>', name: 'Postincrement B-indirect', description: 'Use the addressed instruction\'s B-field as an indirect pointer, then increment it.' },
  { symbol: '{', name: 'Predecrement A-indirect', description: 'Decrement the addressed instruction\'s A-field, then use it as an indirect pointer.' },
  { symbol: '}', name: 'Postincrement A-indirect', description: 'Use the addressed instruction\'s A-field as an indirect pointer, then increment it.' }
];

export const OPCODE_MAP = new Map(OPCODES.map((opcode) => [opcode.name, opcode]));
export const MODIFIER_MAP = new Map(MODIFIERS.map((modifier) => [modifier.name, modifier]));
export const DIRECTIVE_MAP = new Map(DIRECTIVES.map((directive) => [directive.name, directive]));
export const PREDEFINED_CONSTANT_MAP = new Map(PREDEFINED_CONSTANTS.map((constant) => [constant.name, constant]));
export const ADDRESSING_MODE_MAP = new Map(ADDRESSING_MODES.map((mode) => [mode.symbol, mode]));
export const RESERVED_WORDS = new Set([
  ...OPCODES.map((opcode) => opcode.name),
  ...DIRECTIVES.map((directive) => directive.name),
  ...PREDEFINED_CONSTANTS.map((constant) => constant.name),
  'CMP',
  'FOR',
  'ROF',
  'LDP',
  'STP'
]);
