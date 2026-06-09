import * as vscode from 'vscode';
import { registerBattleView } from './battleViewProvider';
import { registerCompletionProvider } from './completionProvider';
import { registerHoverProvider } from './hoverProvider';
import { registerTaskProvider } from './taskProvider';

export function activate(context: vscode.ExtensionContext): void {
  registerCompletionProvider(context);
  registerHoverProvider(context);
  registerTaskProvider(context);
  registerBattleView(context);

  context.subscriptions.push(
    vscode.commands.registerCommand('corewar.validateWarrior', () => {
      void vscode.window.showInformationMessage('Warrior validation is not implemented yet.');
    })
  );
}

export function deactivate(): void {
  // No-op placeholder for future cleanup.
}
