import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

interface HostToWebviewMessage {
  type: 'loadWarrior' | 'play' | 'pause' | 'step' | 'setSpeed' | 'reset';
  source?: string;
  name?: string;
  speed?: number;
}

interface WebviewToHostMessage {
  type: 'ready' | 'battleComplete' | 'error' | 'pickWarrior';
  result?: { cycle: number; winnerId: number | null; survivors: number[] };
  msg?: string;
}

export class BattleViewProvider implements vscode.Disposable {
  private panel: vscode.WebviewPanel | undefined;
  private readonly pendingMessages: HostToWebviewMessage[] = [];
  private isReady = false;

  public constructor(private readonly context: vscode.ExtensionContext) {}

  public openBattleView(autoLoadActiveEditor = true): void {
    if (this.panel === undefined) {
      this.panel = vscode.window.createWebviewPanel(
        'corewarBattleView',
        'CoreWar Battle Viewer',
        vscode.ViewColumn.Beside,
        {
          enableScripts: true,
          retainContextWhenHidden: true,
          localResourceRoots: [
            vscode.Uri.joinPath(this.context.extensionUri, 'dist'),
            vscode.Uri.joinPath(this.context.extensionUri, 'src', 'webview')
          ]
        }
      );

      this.panel.iconPath = new vscode.ThemeIcon('pulse');
      this.panel.onDidDispose(() => {
        this.panel = undefined;
        this.isReady = false;
      }, null, this.context.subscriptions);
      this.panel.webview.onDidReceiveMessage((message: WebviewToHostMessage) => {
        void this.handleWebviewMessage(message);
      }, null, this.context.subscriptions);
      this.panel.webview.html = this.getWebviewHtml(this.panel.webview);
    } else {
      this.panel.reveal(vscode.ViewColumn.Beside, true);
    }

    if (autoLoadActiveEditor) {
      void this.autoloadActiveEditor();
    }
  }

  public play(): void {
    this.postMessage({ type: 'play' });
  }

  public pause(): void {
    this.postMessage({ type: 'pause' });
  }

  public step(): void {
    this.postMessage({ type: 'step' });
  }

  public setSpeed(speed: number): void {
    this.postMessage({ type: 'setSpeed', speed });
  }

  public reset(): void {
    this.postMessage({ type: 'reset' });
  }

  public async addWarrior(): Promise<void> {
    this.openBattleView(this.panel === undefined);

    const pick = await vscode.window.showOpenDialog({
      canSelectMany: false,
      openLabel: 'Add Warrior',
      filters: {
        Redcode: ['red', 'rc']
      }
    });

    const uri = pick?.[0];
    if (uri === undefined) {
      return;
    }

    await this.loadWarriorFromUri(uri);
  }

  public dispose(): void {
    this.panel?.dispose();
  }

  private async handleWebviewMessage(message: WebviewToHostMessage): Promise<void> {
    switch (message.type) {
      case 'ready':
        this.isReady = true;
        while (this.pendingMessages.length > 0) {
          const nextMessage = this.pendingMessages.shift();
          if (nextMessage !== undefined) {
            void this.panel?.webview.postMessage(nextMessage);
          }
        }
        break;
      case 'battleComplete': {
        const winner = message.result?.winnerId;
        const text = winner === null
          ? `Battle complete after ${message.result?.cycle ?? 0} cycles: tie.`
          : `Battle complete after ${message.result?.cycle ?? 0} cycles. Winner: Warrior ${winner}.`;
        void vscode.window.showInformationMessage(text);
        break;
      }
      case 'error':
        if (message.msg !== undefined) {
          void vscode.window.showErrorMessage(`CoreWar battle view: ${message.msg}`);
        }
        break;
      case 'pickWarrior':
        await this.addWarrior();
        break;
      default:
        break;
    }
  }

  private async autoloadActiveEditor(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (editor === undefined) {
      return;
    }

    const isRedcode = editor.document.languageId === 'redcode' || ['.red', '.rc'].includes(path.extname(editor.document.fileName));
    if (!isRedcode) {
      return;
    }

    const source = editor.document.getText();
    const name = path.basename(editor.document.fileName, path.extname(editor.document.fileName));
    this.postMessage({ type: 'loadWarrior', source, name });
  }

  private async loadWarriorFromUri(uri: vscode.Uri): Promise<void> {
    const source = Buffer.from(await vscode.workspace.fs.readFile(uri)).toString('utf8');
    const name = path.basename(uri.fsPath, path.extname(uri.fsPath));
    this.postMessage({ type: 'loadWarrior', source, name });
  }

  private postMessage(message: HostToWebviewMessage): void {
    if (this.panel === undefined || !this.isReady) {
      this.pendingMessages.push(message);
      return;
    }

    void this.panel.webview.postMessage(message);
  }

  private getWebviewHtml(webview: vscode.Webview): string {
    const htmlPath = vscode.Uri.joinPath(this.context.extensionUri, 'src', 'webview', 'battleView.html');
    const html = fs.readFileSync(htmlPath.fsPath, 'utf8');
    const nonce = this.createNonce();

    return html
      .replaceAll('{{cspSource}}', webview.cspSource)
      .replaceAll(
        '{{styleUri}}',
        webview.asWebviewUri(vscode.Uri.joinPath(this.context.extensionUri, 'src', 'webview', 'styles.css')).toString()
      )
      .replaceAll(
        '{{scriptUri}}',
        webview.asWebviewUri(vscode.Uri.joinPath(this.context.extensionUri, 'dist', 'webview.js')).toString()
      )
      .replaceAll('{{nonce}}', nonce);
  }

  private createNonce(): string {
    return Array.from({ length: 32 }, () => Math.floor(Math.random() * 36).toString(36)).join('');
  }
}

export function registerBattleView(context: vscode.ExtensionContext): void {
  const battleViewProvider = new BattleViewProvider(context);

  context.subscriptions.push(
    battleViewProvider,
    vscode.commands.registerCommand('corewar.openBattleView', () => {
      battleViewProvider.openBattleView();
    }),
    vscode.commands.registerCommand('corewar.addWarrior', () => {
      void battleViewProvider.addWarrior();
    })
  );
}
