import * as vscode from 'vscode';
import {
  ADDRESSING_MODES,
  DIRECTIVES,
  DIRECTIVE_MAP,
  MODIFIERS,
  OPCODE_MAP,
  OPCODES,
  PREDEFINED_CONSTANTS,
  RESERVED_WORDS,
  type AddressingModeInfo,
  type OpcodeInfo
} from './redcodeData';

interface LabelInfo {
  name: string;
  line: number;
  instruction: string;
}

function stripInlineComment(line: string): string {
  const commentIndex = line.indexOf(';');
  return commentIndex >= 0 ? line.slice(0, commentIndex) : line;
}

function normalizeLabel(label: string): string {
  return label.replace(/:$/, '').toUpperCase();
}

function isValidLabelToken(token: string): boolean {
  return /^[A-Za-z_][A-Za-z0-9_$]*:?$/.test(token);
}

function parseLabels(document: vscode.TextDocument): LabelInfo[] {
  const labels = new Map<string, LabelInfo>();
  const pendingLabels: string[] = [];

  for (let lineNumber = 0; lineNumber < document.lineCount; lineNumber += 1) {
    const code = stripInlineComment(document.lineAt(lineNumber).text).trim();
    if (!code) {
      continue;
    }

    const colonMatch = code.match(/^([A-Za-z_][A-Za-z0-9_$]*:)(?:\s+(.*))?$/);
    if (colonMatch) {
      const labelName = normalizeLabel(colonMatch[1]);
      const statement = colonMatch[2]?.trim();
      if (statement) {
        labels.set(labelName, { name: labelName, line: lineNumber, instruction: statement });
      } else {
        pendingLabels.push(labelName);
      }
      continue;
    }

    const parts = code.split(/\s+/);
    const first = parts[0];
    const second = parts[1];

    if (parts.length === 1 && isValidLabelToken(first) && !RESERVED_WORDS.has(normalizeLabel(first))) {
      pendingLabels.push(normalizeLabel(first));
      continue;
    }

    let statement = code;
    if (
      parts.length > 1 &&
      isValidLabelToken(first) &&
      !RESERVED_WORDS.has(normalizeLabel(first)) &&
      second !== undefined
    ) {
      const labelName = normalizeLabel(first);
      statement = code.slice(first.length).trimStart();
      labels.set(labelName, { name: labelName, line: lineNumber, instruction: statement });
    }

    for (const pendingLabel of pendingLabels.splice(0)) {
      labels.set(pendingLabel, { name: pendingLabel, line: lineNumber, instruction: statement });
    }
  }

  return Array.from(labels.values()).sort((left, right) => left.name.localeCompare(right.name));
}

function removeLabelPrefix(code: string): string {
  const trimmed = code.trimStart();
  if (!trimmed) {
    return trimmed;
  }

  const colonMatch = trimmed.match(/^[A-Za-z_][A-Za-z0-9_$]*:\s*(.*)$/);
  if (colonMatch) {
    return colonMatch[1] ?? '';
  }

  const wordMatch = trimmed.match(/^([A-Za-z_][A-Za-z0-9_$]*)(\s+)(.*)$/);
  if (wordMatch) {
    const candidate = normalizeLabel(wordMatch[1]);
    if (!RESERVED_WORDS.has(candidate)) {
      return wordMatch[3] ?? '';
    }
  }

  return trimmed;
}

function buildModifierChoice(opcode: OpcodeInfo): string {
  const modifiers = [opcode.defaultModifier, ...opcode.validModifiers.filter((modifier) => modifier !== opcode.defaultModifier)];
  return modifiers.join(',');
}

function buildOpcodeSnippet(opcode: OpcodeInfo): vscode.SnippetString {
  return new vscode.SnippetString(
    `${opcode.name}.\${1|${buildModifierChoice(opcode)}|} \${2:$}\${3:0}, \${4:$}\${5:1}`
  );
}

function createOpcodeCompletion(opcode: OpcodeInfo): vscode.CompletionItem {
  const item = new vscode.CompletionItem(opcode.name, vscode.CompletionItemKind.Keyword);
  item.insertText = buildOpcodeSnippet(opcode);
  item.detail = opcode.description;
  item.documentation = new vscode.MarkdownString(
    'Default modifier: `.' +
      opcode.defaultModifier +
      '`\n\nValid modifiers: ' +
      opcode.validModifiers.map((modifier) => '`.' + modifier + '`').join(', ')
  );
  return item;
}

function createDirectiveCompletion(name: string, description: string, snippet: string): vscode.CompletionItem {
  const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Keyword);
  item.insertText = new vscode.SnippetString(snippet);
  item.detail = description;
  return item;
}

function createModifierCompletion(name: string, description: string): vscode.CompletionItem {
  const item = new vscode.CompletionItem(`.${name}`, vscode.CompletionItemKind.EnumMember);
  item.insertText = name;
  item.detail = description;
  return item;
}

function createConstantCompletion(name: string, value: number, description: string): vscode.CompletionItem {
  const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Constant);
  item.detail = `${description} (${value})`;
  return item;
}

function createAddressingModeCompletion(
  mode: AddressingModeInfo,
  replacementRange?: vscode.Range
): vscode.CompletionItem {
  const item = new vscode.CompletionItem(mode.symbol, vscode.CompletionItemKind.Operator);
  item.insertText = new vscode.SnippetString(`${mode.symbol}\${1:0}`);
  item.detail = `${mode.name}: ${mode.description}`;
  if (replacementRange !== undefined) {
    item.range = replacementRange;
  }
  return item;
}

function createLabelCompletion(label: LabelInfo): vscode.CompletionItem {
  const item = new vscode.CompletionItem(label.name, vscode.CompletionItemKind.Reference);
  item.detail = `Label → ${label.instruction}`;
  return item;
}

function isModifierContext(statementPrefix: string): boolean {
  const match = statementPrefix.match(/^([A-Za-z]+)\.$/);
  return match !== null && OPCODE_MAP.has(match[1].toUpperCase());
}

function isStatementStartContext(statementPrefix: string): boolean {
  return !statementPrefix.trim() || !/[\s.]/.test(statementPrefix);
}

export function registerCompletionProvider(context: vscode.ExtensionContext): void {
  const provider = vscode.languages.registerCompletionItemProvider(
    'redcode',
    {
      provideCompletionItems(document, position): vscode.CompletionItem[] {
        const linePrefix = stripInlineComment(document.lineAt(position.line).text).slice(0, position.character);
        const statementPrefix = removeLabelPrefix(linePrefix).trimStart();

        if (isModifierContext(statementPrefix)) {
          return MODIFIERS.map((modifier) => createModifierCompletion(modifier.name, modifier.description));
        }

        if (isStatementStartContext(statementPrefix)) {
          return [
            ...OPCODES.map((opcode) => createOpcodeCompletion(opcode)),
            ...DIRECTIVES.map((directive) =>
              createDirectiveCompletion(directive.name, directive.description, directive.snippet)
            )
          ];
        }

        const keywordMatch = statementPrefix.match(/^([A-Za-z]+)(?:\.[A-Za-z]*)?(.*)$/);
        const keyword = keywordMatch?.[1]?.toUpperCase();
        const remainder = keywordMatch?.[2] ?? '';

        if (keyword !== undefined && OPCODE_MAP.has(keyword)) {
          const trailingMode = ADDRESSING_MODES.find((mode) => linePrefix.endsWith(mode.symbol));
          const replacementRange =
            trailingMode === undefined
              ? undefined
              : new vscode.Range(position.line, position.character - 1, position.line, position.character);

          return [
            ...ADDRESSING_MODES.map((mode) => createAddressingModeCompletion(mode, replacementRange)),
            ...parseLabels(document).map((label) => createLabelCompletion(label)),
            ...PREDEFINED_CONSTANTS.map((constant) =>
              createConstantCompletion(constant.name, constant.value, constant.description)
            )
          ];
        }

        if (keyword !== undefined && DIRECTIVE_MAP.has(keyword) && remainder.length > 0) {
          return [
            ...parseLabels(document).map((label) => createLabelCompletion(label)),
            ...PREDEFINED_CONSTANTS.map((constant) =>
              createConstantCompletion(constant.name, constant.value, constant.description)
            )
          ];
        }

        return [];
      }
    },
    '.',
    '#',
    '$',
    '@',
    '<',
    '>',
    '{',
    '}'
  );

  context.subscriptions.push(provider);
}
