import * as vscode from 'vscode';
import {
  ADDRESSING_MODE_MAP,
  MODIFIER_MAP,
  OPCODE_MAP,
  PREDEFINED_CONSTANT_MAP,
  RESERVED_WORDS
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

function parseLabels(document: vscode.TextDocument): Map<string, LabelInfo> {
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

  return labels;
}

function findMatchAtPosition(text: string, position: number, expression: RegExp): { start: number; end: number; text: string } | undefined {
  for (const match of text.matchAll(expression)) {
    const start = match.index ?? 0;
    const matchedText = match[0];
    const end = start + matchedText.length;
    if (position >= start && position < end) {
      return { start, end, text: matchedText };
    }
  }

  return undefined;
}

export function registerHoverProvider(context: vscode.ExtensionContext): void {
  const provider = vscode.languages.registerHoverProvider('redcode', {
    provideHover(document, position) {
      const lineText = document.lineAt(position.line).text;
      const commentIndex = lineText.indexOf(';');
      if (commentIndex >= 0 && position.character >= commentIndex) {
        return undefined;
      }

      const code = stripInlineComment(lineText);
      const character = code[position.character] ?? '';
      const lineOffset = position.character;

      const mode = ADDRESSING_MODE_MAP.get(character);
      if (mode !== undefined) {
        return new vscode.Hover(
          new vscode.MarkdownString(`**${mode.symbol} — ${mode.name}**\n\n${mode.description}`),
          new vscode.Range(position, position.translate(0, 1))
        );
      }

      const modifierMatch = findMatchAtPosition(code, lineOffset, /\.(AB|BA|A|B|F|X|I)\b/gi);
      if (modifierMatch !== undefined) {
        const modifierName = modifierMatch.text.slice(1).toUpperCase();
        const modifier = MODIFIER_MAP.get(modifierName);
        if (modifier !== undefined) {
          return new vscode.Hover(
            new vscode.MarkdownString(`**.${modifier.name} modifier**\n\n${modifier.description}`),
            new vscode.Range(position.line, modifierMatch.start, position.line, modifierMatch.end)
          );
        }
      }

      const opcodePattern = new RegExp(`\\b(${Array.from(OPCODE_MAP.keys()).join('|')})\\b`, 'gi');
      const opcodeMatch = findMatchAtPosition(code, lineOffset, opcodePattern);
      if (opcodeMatch !== undefined) {
        const opcode = OPCODE_MAP.get(opcodeMatch.text.toUpperCase());
        if (opcode !== undefined) {
          const markdown = new vscode.MarkdownString();
          markdown.appendMarkdown(`**${opcode.name}**\n\n${opcode.description}\n\n`);
          markdown.appendMarkdown(`Default modifier: ${'`'}.${opcode.defaultModifier}${'`'}\n\n`);
          markdown.appendMarkdown('Valid modifiers:\n');
          for (const modifierName of opcode.validModifiers) {
            const modifier = MODIFIER_MAP.get(modifierName);
            const description = modifier?.description ?? '';
            markdown.appendMarkdown(`- ${'`'}.${modifierName}${'`'} — ${description}\n`);
          }

          return new vscode.Hover(
            markdown,
            new vscode.Range(position.line, opcodeMatch.start, position.line, opcodeMatch.end)
          );
        }
      }

      const constantPattern = new RegExp(`\\b(${Array.from(PREDEFINED_CONSTANT_MAP.keys()).join('|')})\\b`, 'gi');
      const constantMatch = findMatchAtPosition(code, lineOffset, constantPattern);
      if (constantMatch !== undefined) {
        const constant = PREDEFINED_CONSTANT_MAP.get(constantMatch.text.toUpperCase());
        if (constant !== undefined) {
          return new vscode.Hover(
            new vscode.MarkdownString(
              `**${constant.name}** = ${'`'}${constant.value}${'`'}\n\n${constant.description}`
            ),
            new vscode.Range(position.line, constantMatch.start, position.line, constantMatch.end)
          );
        }
      }

      const labelMatch = findMatchAtPosition(code, lineOffset, /\b[A-Za-z_][A-Za-z0-9_$]*\b/g);
      if (labelMatch === undefined) {
        return undefined;
      }

      const labelName = normalizeLabel(labelMatch.text);
      if (RESERVED_WORDS.has(labelName)) {
        return undefined;
      }

      const label = parseLabels(document).get(labelName);
      if (label === undefined) {
        return undefined;
      }

      const markdown = new vscode.MarkdownString();
      markdown.appendMarkdown(`**Label ${label.name}**\n\n`);
      markdown.appendMarkdown(`Points to line ${label.line + 1}:\n`);
      markdown.appendCodeblock(label.instruction, 'redcode');

      return new vscode.Hover(
        markdown,
        new vscode.Range(position.line, labelMatch.start, position.line, labelMatch.end)
      );
    }
  });

  context.subscriptions.push(provider);
}
