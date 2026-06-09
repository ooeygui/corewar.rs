import * as vscode from 'vscode';
import { buildParserProblemTransformScript, COREWAR_PROBLEM_MATCHER } from './problemMatcher';

const COREWAR_TASK_TYPE = 'corewar';
const CURRENT_FILE_VARIABLE = '${file}';
const RELATIVE_FILE_VARIABLE = '${relativeFile}';

type CorewarTaskKind = 'validate' | 'battle' | 'build-wasm';

export interface CorewarTaskDefinition extends vscode.TaskDefinition {
  task: CorewarTaskKind;
  warriors?: string[];
  coreSize?: number;
  maxCycles?: number;
}

class CorewarTaskProvider implements vscode.TaskProvider {
  provideTasks(): vscode.ProviderResult<vscode.Task[]> {
    const folders = vscode.workspace.workspaceFolders ?? [];
    return folders.flatMap((folder) => this.createTasksForFolder(folder));
  }

  resolveTask(task: vscode.Task): vscode.ProviderResult<vscode.Task> {
    const definition = task.definition as Partial<CorewarTaskDefinition>;
    if (!isCorewarTaskDefinition(definition)) {
      return undefined;
    }

    const folder = isWorkspaceFolder(task.scope)
      ? task.scope
      : vscode.workspace.workspaceFolders?.[0];

    if (!folder) {
      return undefined;
    }

    return this.createTask(folder, definition);
  }

  private createTasksForFolder(folder: vscode.WorkspaceFolder): vscode.Task[] {
    return [
      this.createTask(folder, { type: COREWAR_TASK_TYPE, task: 'validate' }),
      this.createTask(folder, { type: COREWAR_TASK_TYPE, task: 'battle', warriors: [CURRENT_FILE_VARIABLE] }),
      this.createTask(folder, { type: COREWAR_TASK_TYPE, task: 'build-wasm' })
    ];
  }

  private createTask(folder: vscode.WorkspaceFolder, definition: CorewarTaskDefinition): vscode.Task {
    const task = new vscode.Task(
      definition,
      folder,
      this.getTaskName(definition),
      COREWAR_TASK_TYPE,
      this.createExecution(definition, folder),
      this.getProblemMatchers(definition)
    );

    task.detail = this.getTaskDetail(definition);
    task.presentationOptions = {
      clear: true,
      reveal: vscode.TaskRevealKind.Always
    };
    task.runOptions = {
      reevaluateOnRerun: true
    };

    if (definition.task === 'build-wasm') {
      task.group = vscode.TaskGroup.Build;
    }

    return task;
  }

  private createExecution(
    definition: CorewarTaskDefinition,
    folder: vscode.WorkspaceFolder
  ): vscode.ShellExecution {
    const options: vscode.ShellExecutionOptions = {
      cwd: folder.uri.fsPath
    };

    switch (definition.task) {
      case 'validate':
        return new vscode.ShellExecution('powershell', [
          '-NoProfile',
          '-ExecutionPolicy',
          'Bypass',
          '-Command',
          buildValidateCommand()
        ], options);
      case 'battle': {
        const args = ['run', '-p', 'corewar-vm', '--', 'battle'];
        if (definition.coreSize !== undefined) {
          args.push('--core-size', String(definition.coreSize));
        }
        if (definition.maxCycles !== undefined) {
          args.push('--max-cycles', String(definition.maxCycles));
        }
        args.push(...getBattleWarriors(definition));
        return new vscode.ShellExecution('cargo', args, options);
      }
      case 'build-wasm':
        return new vscode.ShellExecution('wasm-pack', [
          'build',
          'crates/corewar-viz',
          '--target',
          'web',
          '--features',
          'wasm'
        ], options);
    }
  }

  private getProblemMatchers(definition: CorewarTaskDefinition): string[] {
    return definition.task === 'validate' ? [COREWAR_PROBLEM_MATCHER] : [];
  }

  private getTaskDetail(definition: CorewarTaskDefinition): string {
    switch (definition.task) {
      case 'validate':
        return 'Validate the active Redcode warrior with corewar-parser.';
      case 'battle':
        return 'Run a CoreWar battle with the configured warriors.';
      case 'build-wasm':
        return 'Build the corewar-viz WASM bundle with wasm-pack.';
    }
  }

  private getTaskName(definition: CorewarTaskDefinition): string {
    switch (definition.task) {
      case 'validate':
        return 'Validate Warrior';
      case 'battle':
        return 'Run Battle';
      case 'build-wasm':
        return 'Build WASM';
    }
  }
}

function buildValidateCommand(): string {
  const formatter = buildParserProblemTransformScript(RELATIVE_FILE_VARIABLE);
  return [
    `$ErrorActionPreference = 'Continue'`,
    `cargo run -p corewar-parser -- validate "${CURRENT_FILE_VARIABLE}" 2>&1 | & { ${formatter} }`,
    'exit $LASTEXITCODE'
  ].join('; ');
}

function getBattleWarriors(definition: CorewarTaskDefinition): string[] {
  return definition.warriors && definition.warriors.length > 0
    ? definition.warriors
    : [CURRENT_FILE_VARIABLE];
}

function isCorewarTaskDefinition(definition: Partial<CorewarTaskDefinition>): definition is CorewarTaskDefinition {
  return definition.type === COREWAR_TASK_TYPE
    && (definition.task === 'validate' || definition.task === 'battle' || definition.task === 'build-wasm');
}

function isWorkspaceFolder(scope: vscode.TaskScope | vscode.WorkspaceFolder | undefined): scope is vscode.WorkspaceFolder {
  return scope !== undefined && typeof scope !== 'number' && 'uri' in scope;
}

export function registerTaskProvider(context: vscode.ExtensionContext): void {
  const provider = vscode.tasks.registerTaskProvider(COREWAR_TASK_TYPE, new CorewarTaskProvider());
  context.subscriptions.push(provider);
}
